use std::collections::HashMap;

use openaction::*;
use tokio::process::Command;

struct UsageAction;
#[async_trait]
impl Action for UsageAction {
	const UUID: ActionUuid = "dev.maugap.oagpustats.usage";
	type Settings = HashMap<String, String>;
}

struct TemperatureAction;
#[async_trait]
impl Action for TemperatureAction {
	const UUID: ActionUuid = "dev.maugap.oagpustats.temperature";
	type Settings = HashMap<String, String>;
}

struct MemoryAction;
#[async_trait]
impl Action for MemoryAction {
	const UUID: ActionUuid = "dev.maugap.oagpustats.memory";
	type Settings = HashMap<String, String>;
}

struct PowerAction;
#[async_trait]
impl Action for PowerAction {
	const UUID: ActionUuid = "dev.maugap.oagpustats.power";
	type Settings = HashMap<String, String>;
}

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

	Some(GpuSnapshot {
		usage_pct: fields.next().and_then(parse),
		temp_c: fields.next().and_then(parse),
		mem_used_mib: fields.next().and_then(parse),
		mem_total_mib: fields.next().and_then(parse),
		power_w: fields.next().and_then(parse),
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

			let (usage, temp, mem, power) = match snap {
				Some(s) => (
					fmt_opt(s.usage_pct, |v| format!("{:.0}%", v)),
					fmt_opt(s.temp_c, |v| format!("{:.0}°C", v)),
					match (s.mem_used_mib, s.mem_total_mib) {
						(Some(u), Some(t)) => format!("{:.1}/{:.0}\nGB", u / 1024.0, t / 1024.0),
						(Some(u), None) => format!("{:.1}GB", u / 1024.0),
						_ => "N/A".to_string(),
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
