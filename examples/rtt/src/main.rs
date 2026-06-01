#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_mcxa::{
    Peri,
    adc::{self, Adc, Command, CommandConfig, CommandId, Trigger},
    bind_interrupts,
    clocks::config::Div8,
    gpio::{DriveStrength, Level, Output, SlewRate},
    peripherals::{self, ADC1, P1_14, P1_15},
};
use embassy_time::{Duration, Ticker, Timer};
use embedded_perfmon_runtime as _;
use embedded_perfmon_transport::Event;
use panic_probe as _;
use rtt_target::UpChannel;

bind_interrupts!(struct Irqs {
    ADC1 => adc::InterruptHandler<peripherals::ADC1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let channels = rtt_target::rtt_init! {
        up: {
            0: {
                size: 1024,
                mode: rtt_target::ChannelMode::NoBlockSkip,
                name: "defmt"
            }
            1: {
                size: 1024,
                mode: rtt_target::ChannelMode::NoBlockSkip,
                name: "trace"
            }
        }
    };
    rtt_target::set_defmt_channel(channels.up.0);
    unsafe {
        TRACE_CHANNEL = Some(channels.up.1);
    }

    embedded_perfmon_runtime::emit_tickrate_trace();

    let mut config = embassy_mcxa::config::Config::default();
    config.clock_cfg.sirc.fro_lf_div = Div8::from_divisor(1);
    let p = embassy_mcxa::init(config);

    spawner.spawn(measure_adc(p.ADC1, p.P1_14, p.P1_15).unwrap());

    let mut red = Output::new(p.P3_18, Level::High, DriveStrength::Normal, SlewRate::Fast);
    let mut green = Output::new(p.P3_19, Level::High, DriveStrength::Normal, SlewRate::Fast);
    let mut blue = Output::new(p.P3_21, Level::High, DriveStrength::Normal, SlewRate::Fast);

    loop {
        defmt::info!("Toggle LEDs");

        red.toggle();
        Timer::after_millis(250).await;

        red.toggle();
        green.toggle();
        Timer::after_millis(250).await;

        green.toggle();
        blue.toggle();
        Timer::after_millis(250).await;
        blue.toggle();

        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
async fn measure_adc(
    adc1: Peri<'static, ADC1>,
    pin1: Peri<'static, P1_14>,
    pin2: Peri<'static, P1_15>,
) {
    let commands = &[
        Command::new_single(
            pin1,
            CommandConfig {
                chained_command: Some(CommandId::Cmd2), // Command 2 is executed after this command is done
                ..Default::default()
            },
        ),
        Command::new_looping(
            pin2,
            3, // Command is run 3 times
            CommandConfig {
                chained_command: None, // Terminate the conversion after command is done
                ..Default::default()
            },
        )
        .unwrap(),
    ];

    let mut adc = Adc::new_async(
        adc1,
        Irqs,
        commands,
        &[Trigger {
            target_command_id: CommandId::Cmd1,
            enable_hardware_trigger: false,
            ..Default::default()
        }],
        adc::Config::default(),
    )
    .unwrap();

    adc.do_offset_calibration();
    adc.do_auto_calibration();

    defmt::info!("=== ADC configuration done... ===");
    let mut tick = Ticker::every(Duration::from_millis(1000));

    loop {
        tick.next().await;
        adc.do_software_trigger(0b0001).unwrap();

        while let Some(res) = adc.wait_get_conversion().await {
            defmt::info!("ADC result: {}", res);
        }
    }
}

static mut TRACE_CHANNEL: Option<UpChannel> = None;

#[unsafe(no_mangle)]
fn _write_trace_event(event: Event<'_>) {
    let mut buf = [0; 128];
    unsafe {
        #[allow(static_mut_refs)] // TODO: Make safer
        if let Some(c) = TRACE_CHANNEL.as_mut() {
            c.write(event.serialize(&mut buf).unwrap());
        }
    }
}

#[unsafe(no_mangle)]
fn _get_trace_event_timestamp() -> u64 {
    embassy_time::Instant::now().as_ticks()
}

#[unsafe(no_mangle)]
fn _get_trace_event_tickrate() -> u64 {
    embassy_time::TICK_HZ
}
