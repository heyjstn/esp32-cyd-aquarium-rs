#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitCommand {
    pub command: u8,
    pub args: &'static [u8],
    pub delay_after_ms: u16,
}

const POWER_CONTROL_B: u8 = 0xCF;
const POWER_ON_SEQUENCE_CONTROL: u8 = 0xED;
const DRIVER_TIMING_CONTROL_A: u8 = 0xE8;
const POWER_CONTROL_A: u8 = 0xCB;
const PUMP_RATIO_CONTROL: u8 = 0xF7;
const DRIVER_TIMING_CONTROL_B: u8 = 0xEA;
const POWER_CONTROL_1: u8 = 0xC0;
const POWER_CONTROL_2: u8 = 0xC1;
const VCOM_CONTROL_1: u8 = 0xC5;
const VCOM_CONTROL_2: u8 = 0xC7;
const PIXEL_FORMAT: u8 = 0x3A;
const MEMORY_ACCESS_CONTROL: u8 = 0x36;
const FRAME_RATE_CONTROL: u8 = 0xB1;
const DISPLAY_FUNCTION_CONTROL: u8 = 0xB6;
const ENABLE_3_GAMMA: u8 = 0xF2;
const GAMMA_SET: u8 = 0x26;
const POSITIVE_GAMMA_CORRECTION: u8 = 0xE0;
const NEGATIVE_GAMMA_CORRECTION: u8 = 0xE1;
const PAGE_ADDRESS_SET: u8 = 0x2B;
const COLUMN_ADDRESS_SET: u8 = 0x2A;
const SLEEP_OUT: u8 = 0x11;
const DISPLAY_ON: u8 = 0x29;
const INVERSION_OFF: u8 = 0x20;

pub const RESET_DELAY_MS: u16 = 150;
pub const SLEEP_OUT_DELAY_MS: u16 = 120;
pub const RGB565_PIXEL_FORMAT: u8 = 0x55;
pub const BGR_MEMORY_ORDER: u8 = 0x08;
pub const ROTATION_2_BGR_MEMORY_ORDER: u8 = 0x88;
pub const ROTATION_2: InitCommand = InitCommand {
    command: MEMORY_ACCESS_CONTROL,
    args: &[ROTATION_2_BGR_MEMORY_ORDER],
    delay_after_ms: 0,
};

pub const ILI9341_2_INIT: &[InitCommand] = &[
    InitCommand {
        command: POWER_CONTROL_B,
        args: &[0x00, 0xC1, 0x30],
        delay_after_ms: 0,
    },
    InitCommand {
        command: POWER_ON_SEQUENCE_CONTROL,
        args: &[0x64, 0x03, 0x12, 0x81],
        delay_after_ms: 0,
    },
    InitCommand {
        command: DRIVER_TIMING_CONTROL_A,
        args: &[0x85, 0x00, 0x78],
        delay_after_ms: 0,
    },
    InitCommand {
        command: POWER_CONTROL_A,
        args: &[0x39, 0x2C, 0x00, 0x34, 0x02],
        delay_after_ms: 0,
    },
    InitCommand {
        command: PUMP_RATIO_CONTROL,
        args: &[0x20],
        delay_after_ms: 0,
    },
    InitCommand {
        command: DRIVER_TIMING_CONTROL_B,
        args: &[0x00, 0x00],
        delay_after_ms: 0,
    },
    InitCommand {
        command: POWER_CONTROL_1,
        args: &[0x10],
        delay_after_ms: 0,
    },
    InitCommand {
        command: POWER_CONTROL_2,
        args: &[0x00],
        delay_after_ms: 0,
    },
    InitCommand {
        command: VCOM_CONTROL_1,
        args: &[0x30, 0x30],
        delay_after_ms: 0,
    },
    InitCommand {
        command: VCOM_CONTROL_2,
        args: &[0xB7],
        delay_after_ms: 0,
    },
    InitCommand {
        command: PIXEL_FORMAT,
        args: &[RGB565_PIXEL_FORMAT],
        delay_after_ms: 0,
    },
    InitCommand {
        command: MEMORY_ACCESS_CONTROL,
        args: &[BGR_MEMORY_ORDER],
        delay_after_ms: 0,
    },
    InitCommand {
        command: FRAME_RATE_CONTROL,
        args: &[0x00, 0x1A],
        delay_after_ms: 0,
    },
    InitCommand {
        command: DISPLAY_FUNCTION_CONTROL,
        args: &[0x08, 0x82, 0x27],
        delay_after_ms: 0,
    },
    InitCommand {
        command: ENABLE_3_GAMMA,
        args: &[0x00],
        delay_after_ms: 0,
    },
    InitCommand {
        command: GAMMA_SET,
        args: &[0x01],
        delay_after_ms: 0,
    },
    InitCommand {
        command: POSITIVE_GAMMA_CORRECTION,
        args: &[
            0x0F, 0x2A, 0x28, 0x08, 0x0E, 0x08, 0x54, 0xA9, 0x43, 0x0A, 0x0F, 0x00, 0x00, 0x00,
            0x00,
        ],
        delay_after_ms: 0,
    },
    InitCommand {
        command: NEGATIVE_GAMMA_CORRECTION,
        args: &[
            0x00, 0x15, 0x17, 0x07, 0x11, 0x06, 0x2B, 0x56, 0x3C, 0x05, 0x10, 0x0F, 0x3F, 0x3F,
            0x0F,
        ],
        delay_after_ms: 0,
    },
    InitCommand {
        command: PAGE_ADDRESS_SET,
        args: &[0x00, 0x00, 0x01, 0x3F],
        delay_after_ms: 0,
    },
    InitCommand {
        command: COLUMN_ADDRESS_SET,
        args: &[0x00, 0x00, 0x00, 0xEF],
        delay_after_ms: 0,
    },
    InitCommand {
        command: SLEEP_OUT,
        args: &[],
        delay_after_ms: SLEEP_OUT_DELAY_MS,
    },
    InitCommand {
        command: DISPLAY_ON,
        args: &[],
        delay_after_ms: 0,
    },
    InitCommand {
        command: INVERSION_OFF,
        args: &[],
        delay_after_ms: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn args_for(command: u8) -> &'static [u8] {
        ILI9341_2_INIT
            .iter()
            .find(|step| step.command == command)
            .expect("command is present")
            .args
    }

    #[test]
    fn sequence_matches_ili9341_2_command_order() {
        let commands: Vec<_> = ILI9341_2_INIT.iter().map(|step| step.command).collect();
        assert_eq!(
            commands,
            [
                0xCF, 0xED, 0xE8, 0xCB, 0xF7, 0xEA, 0xC0, 0xC1, 0xC5, 0xC7, 0x3A, 0x36, 0xB1, 0xB6,
                0xF2, 0x26, 0xE0, 0xE1, 0x2B, 0x2A, 0x11, 0x29, 0x20,
            ]
        );
        assert_eq!(args_for(POWER_CONTROL_B), [0x00, 0xC1, 0x30]);
        assert_eq!(
            args_for(POWER_ON_SEQUENCE_CONTROL),
            [0x64, 0x03, 0x12, 0x81]
        );
        assert_eq!(args_for(DRIVER_TIMING_CONTROL_A), [0x85, 0x00, 0x78]);
        assert_eq!(args_for(POWER_CONTROL_A), [0x39, 0x2C, 0x00, 0x34, 0x02]);
        assert_eq!(args_for(PUMP_RATIO_CONTROL), [0x20]);
        assert_eq!(args_for(DRIVER_TIMING_CONTROL_B), [0x00, 0x00]);
        assert_eq!(args_for(POWER_CONTROL_1), [0x10]);
        assert_eq!(args_for(POWER_CONTROL_2), [0x00]);
        assert_eq!(args_for(VCOM_CONTROL_1), [0x30, 0x30]);
        assert_eq!(args_for(VCOM_CONTROL_2), [0xB7]);
        assert_eq!(args_for(FRAME_RATE_CONTROL), [0x00, 0x1A]);
        assert_eq!(args_for(DISPLAY_FUNCTION_CONTROL), [0x08, 0x82, 0x27]);
        assert_eq!(args_for(ENABLE_3_GAMMA), [0x00]);
        assert_eq!(args_for(GAMMA_SET), [0x01]);
        assert_eq!(args_for(PAGE_ADDRESS_SET), [0x00, 0x00, 0x01, 0x3F]);
        assert_eq!(args_for(COLUMN_ADDRESS_SET), [0x00, 0x00, 0x00, 0xEF]);
    }

    #[test]
    fn sequence_preserves_panel_transport_options() {
        assert_eq!(args_for(PIXEL_FORMAT), [RGB565_PIXEL_FORMAT]);
        assert_eq!(args_for(MEMORY_ACCESS_CONTROL), [BGR_MEMORY_ORDER]);
        assert_eq!(
            ROTATION_2,
            InitCommand {
                command: MEMORY_ACCESS_CONTROL,
                args: &[0x88],
                delay_after_ms: 0,
            }
        );
        assert_eq!(ILI9341_2_INIT.last().unwrap().command, INVERSION_OFF);
        assert_eq!(
            ILI9341_2_INIT
                .iter()
                .find(|step| step.command == SLEEP_OUT)
                .unwrap()
                .delay_after_ms,
            SLEEP_OUT_DELAY_MS
        );
    }

    #[test]
    fn sequence_preserves_alternative_gamma_curves() {
        assert_eq!(
            args_for(POSITIVE_GAMMA_CORRECTION),
            [
                0x0F, 0x2A, 0x28, 0x08, 0x0E, 0x08, 0x54, 0xA9, 0x43, 0x0A, 0x0F, 0x00, 0x00, 0x00,
                0x00,
            ]
        );
        assert_eq!(
            args_for(NEGATIVE_GAMMA_CORRECTION),
            [
                0x00, 0x15, 0x17, 0x07, 0x11, 0x06, 0x2B, 0x56, 0x3C, 0x05, 0x10, 0x0F, 0x3F, 0x3F,
                0x0F,
            ]
        );
    }
}
