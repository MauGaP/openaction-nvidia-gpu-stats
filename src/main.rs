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
}

impl Default for BgSettings {
	fn default() -> Self {
		Self {
			color1: "#1e1e1e".to_string(),
			color2: "#444444".to_string(),
			gradient: false,
		}
	}
}

fn render_bg_data_url(s: &BgSettings) -> String {
	let svg = if s.gradient {
		format!(
			r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><defs><linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient></defs><rect width="144" height="144" fill="url(#g)"/></svg>"##,
			s.color1, s.color2
		)
	} else {
		format!(
			r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" fill="{}"/></svg>"##,
			s.color1
		)
	};
	format!(
		"data:image/svg+xml;base64,{}",
		base64::engine::general_purpose::STANDARD.encode(svg)
	)
}

async fn paint_bg(instance: &Instance, settings: &BgSettings) -> OpenActionResult<()> {
	instance
		.set_image(Some(render_bg_data_url(settings)), None)
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
				paint_bg(instance, settings).await
			}

			async fn did_receive_settings(
				&self,
				instance: &Instance,
				settings: &Self::Settings,
			) -> OpenActionResult<()> {
				paint_bg(instance, settings).await
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
	let _mem_total_mib = fields.next().and_then(parse);
	let power_w = fields.next().and_then(parse);

	Some(GpuSnapshot { usage_pct, temp_c, mem_used_mib, power_w })
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

			let (usage, temp, mem, power) = match snap {
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

			for instance in visible_instances(UsageAction::UUID).await {
				let _ = instance.set_title(Some(usage.clone()), None).await;
			}
			for instance in visible_instances(TemperatureAction::UUID).await {
				let _ = instance.set_title(Some(temp.clone()), None).await;
			}
			for instance in visible_instances(MemoryAction::UUID).await {
				let _ = instance.set_title(Some(mem.clone()), None).await;
			}
			for instance in visible_instances(PowerAction::UUID).await {
				let _ = instance.set_title(Some(power.clone()), None).await;
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
