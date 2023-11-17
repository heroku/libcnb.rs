use libcnb::data::exec_d::ExecDProgramOutputKey;
use libcnb::data::exec_d_program_output_key;
use libcnb::exec_d::write_exec_d_program_output;
use std::collections::HashMap;
use std::iter;

// Suppress warnings due to the `unused_crate_dependencies` lint not handling integration tests well.
#[cfg(test)]
use libcnb_test as _;

fn main() {
    write_exec_d_program_output(env_vars());
}

fn env_vars() -> HashMap<ExecDProgramOutputKey, String> {
    HashMap::from([
        (
            exec_d_program_output_key!("ROLL_1D6"),
            roll_dice(1, 6).to_string(),
        ),
        (
            exec_d_program_output_key!("ROLL_4D6"),
            roll_dice(4, 6).to_string(),
        ),
        (
            exec_d_program_output_key!("ROLL_1D20"),
            roll_dice(1, 20).to_string(),
        ),
    ])
}

fn roll_dice(amount: usize, sides: u32) -> u32 {
    iter::repeat_with(|| fastrand::u32(1..=sides))
        .take(amount)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_dice() {
        assert!((1..=6).contains(&roll_dice(1, 6)));
        assert!((8..=32).contains(&roll_dice(8, 4)));
    }

    #[test]
    fn test_env_vars() {
        let env_vars = env_vars();

        let roll_1d6_value = env_vars
            .get("ROLL_1D6")
            .map(|value| value.parse::<u32>().unwrap());

        let roll_4d6_value = env_vars
            .get("ROLL_4D6")
            .map(|value| value.parse::<u32>().unwrap());

        let roll_1d20_value = env_vars
            .get("ROLL_1D20")
            .map(|value| value.parse::<u32>().unwrap());

        assert!(roll_1d6_value.map_or(false, |value| (1..=6).contains(&value)));
        assert!(roll_4d6_value.map_or(false, |value| (4..=32).contains(&value)));
        assert!(roll_1d20_value.map_or(false, |value| (1..=20).contains(&value)));
    }
}
