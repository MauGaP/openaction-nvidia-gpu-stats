use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use openaction::*;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct BgSettings {
	color1: String,
	color2: String,
	gradient: bool,
	balance: u8,
	bar: String,
	bar_color: String,
}

impl Default for BgSettings {
	fn default() -> Self {
		Self {
			color1: "#1e1e1e".to_string(),
			color2: "#444444".to_string(),
			gradient: false,
			balance: 50,
			bar: "none".to_string(),
			bar_color: "#22c55e".to_string(),
		}
	}
}

fn cache() -> &'static Mutex<HashMap<String, BgSettings>> {
	static C: OnceLock<Mutex<HashMap<String, BgSettings>>> = OnceLock::new();
	C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_settings(id: &str) -> BgSettings {
	cache().lock().unwrap().get(id).cloned().unwrap_or_default()
}

fn store_settings(id: &str, s: BgSettings) {
	cache().lock().unwrap().insert(id.to_string(), s);
}

fn render_overlay(pct: f32, style: &str, bar_color: &str) -> String {
	let pct = pct.clamp(0.0, 1.0);
	match style {
		"bar_bottom" => {
			let filled = 144.0 * pct;
			format!(
				r##"<rect x="0" y="130" width="144" height="14" fill="rgba(0,0,0,0.35)"/><rect x="0" y="130" width="{:.1}" height="14" fill="{}"/>"##,
				filled, bar_color
			)
		}
		"arc" => {
			let r: f32 = 60.0;
			let circ = 2.0 * std::f32::consts::PI * r;
			let dash_filled = circ * pct;
			let dash_gap = circ - dash_filled;
			format!(
				r##"<g transform="rotate(-90 72 72)"><circle cx="72" cy="72" r="60" fill="none" stroke="rgba(0,0,0,0.35)" stroke-width="12"/><circle cx="72" cy="72" r="60" fill="none" stroke="{}" stroke-width="12" stroke-dasharray="{:.2} {:.2}" stroke-linecap="round"/></g>"##,
				bar_color, dash_filled, dash_gap
			)
		}
		_ => String::new(),
	}
}

fn render_image_data_url(s: &BgSettings, pct: Option<f32>) -> String {
	let (defs, bg_fill) = if s.gradient {
		let b = (s.balance.min(100) as f32) / 100.0;
		let stop1 = b * 50.0;
		let stop2 = b * 50.0 + 50.0;
		(
			format!(
				r##"<defs><radialGradient id="g" cx="50%" cy="50%" r="70%"><stop offset="{:.1}%" stop-color="{}"/><stop offset="{:.1}%" stop-color="{}"/></radialGradient></defs>"##,
				stop1, s.color1, stop2, s.color2
			),
			"url(#g)".to_string(),
		)
	} else {
		(String::new(), s.color1.clone())
	};

	let overlay = pct
		.map(|p| render_overlay(p, &s.bar, &s.bar_color))
		.unwrap_or_default();

	let svg = format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144">{}<rect width="144" height="144" fill="{}"/>{}</svg>"##,
		defs, bg_fill, overlay
	);

	format!(
		"data:image/svg+xml;base64,{}",
		base64::engine::general_purpose::STANDARD.encode(svg)
	)
}

async fn paint(
	instance: &Instance,
	settings: &BgSettings,
	pct: Option<f32>,
) -> OpenActionResult<()> {
	instance
		.set_image(Some(render_image_data_url(settings, pct)), None)
		.await
}

macro_rules! gpu_action {
	($name:ident, $uuid:expr) => {
		struct $name;
		#[async_trait]
		impl Action for $name {
			const UUID: ActionUuid = $uuid;
			type Settings = BgSettings;

			async fn will_appear(
				&self,
				instance: &Instance,
				settings: &Self::Settings,
			) -> OpenActionResult<()> {
				store_settings(&instance.instance_id, settings.clone());
				paint(instance, settings, None).await
			}

			async fn did_receive_settings(
				&self,
				instance: &Instance,
				settings: &Self::Settings,
			) -> OpenActionResult<()> {
				store_settings(&instance.instance_id, settings.clone());
				paint(instance, settings, None).await
			}

			async fn will_disappear(
				&self,
				instance: &Instance,
				_settings: &Self::Settings,
			) -> OpenActionResult<()> {
				cache().lock().unwrap().remove(&instance.instance_id);
				Ok(())
			}
		}
	};
}

gpu_action!(UsageAction, "dev.maugap.oagpustats.usage");
gpu_action!(TemperatureAction, "dev.maugap.oagpustats.temperature");
gpu_action!(MemoryAction, "dev.maugap.oagpustats.memory");
gpu_action!(PowerAction, "dev.maugap.oagpustats.power");

struct GpuSnapshot {
	usage_pct: Option<f32>,
	temp_c: Option<f32>,
	mem_used_mib: Option<f32>,
	mem_total_mib: Option<f32>,
	power_w: Option<f32>,
}

async fn read_gpu() -> Option<GpuSnapshot> {
	let output = Command::new("nvidia-smi")
		.args([
			"--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total,power.draw",
			"--format=csv,noheader,nounits",
		])
		.output()
		.await
		.ok()?;

	if !output.status.success() {
		return None;
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let first_line = stdout.lines().next()?;
	let mut fields = first_line.split(',').map(|s| s.trim());

	let parse = |v: &str| -> Option<f32> {
		if v == "[N/A]" || v.is_empty() {
			None
		} else {
			v.parse::<f32>().ok()
		}
	};

	let usage_pct = fields.next().and_then(parse);
	let temp_c = fields.next().and_then(parse);
	let mem_used_mib = fields.next().and_then(parse);
	let mem_total_mib = fields.next().and_then(parse);
	let power_w = fields.next().and_then(parse);

	Some(GpuSnapshot {
		usage_pct,
		temp_c,
		mem_used_mib,
		mem_total_mib,
		power_w,
	})
}

fn fmt_opt<F: FnOnce(f32) -> String>(v: Option<f32>, f: F) -> String {
	match v {
		Some(x) => f(x),
		None => "N/A".to_string(),
	}
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
	{
		use simplelog::*;
		if let Err(error) = TermLogger::init(
			LevelFilter::Info,
			Config::default(),
			TerminalMode::Stdout,
			ColorChoice::Never,
		) {
			eprintln!("Logger initialization failed: {}", error);
		}
	}

	tokio::spawn(async {
		loop {
			let snap = read_gpu().await;

			let (usage_title, temp_title, mem_title, power_title) = match &snap {
				Some(s) => (
					fmt_opt(s.usage_pct, |v| format!("{:.0}%", v)),
					fmt_opt(s.temp_c, |v| format!("{:.0}°C", v)),
					match s.mem_used_mib {
						Some(u) if u < 1024.0 => format!("{:.0}MB", u),
						Some(u) => format!("{:.1}GB", u / 1024.0),
						None => "N/A".to_string(),
					},
					fmt_opt(s.power_w, |v| format!("{:.0}W", v)),
				),
				None => (
					"N/A".to_string(),
					"N/A".to_string(),
					"N/A".to_string(),
					"N/A".to_string(),
				),
			};

			let usage_pct = snap.as_ref().and_then(|s| s.usage_pct).map(|v| v / 100.0);
			let mem_pct = snap
				.as_ref()
				.and_then(|s| match (s.mem_used_mib, s.mem_total_mib) {
					(Some(u), Some(t)) if t > 0.0 => Some(u / t),
					_ => None,
				});

			for instance in visible_instances(UsageAction::UUID).await {
				let s = get_settings(&instance.instance_id);
				if s.bar != "none" {
					let _ = instance
						.set_image(Some(render_image_data_url(&s, usage_pct)), None)
						.await;
				}
				let _ = instance.set_title(Some(usage_title.clone()), None).await;
			}

			for instance in visible_instances(MemoryAction::UUID).await {
				let s = get_settings(&instance.instance_id);
				if s.bar != "none" {
					let _ = instance
						.set_image(Some(render_image_data_url(&s, mem_pct)), None)
						.await;
				}
				let _ = instance.set_title(Some(mem_title.clone()), None).await;
			}

			for instance in visible_instances(TemperatureAction::UUID).await {
				let _ = instance.set_title(Some(temp_title.clone()), None).await;
			}

			for instance in visible_instances(PowerAction::UUID).await {
				let _ = instance.set_title(Some(power_title.clone()), None).await;
			}

			tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		}
	});

	register_action(UsageAction).await;
	register_action(TemperatureAction).await;
	register_action(MemoryAction).await;
	register_action(PowerAction).await;

	run(std::env::args().collect()).await
}
