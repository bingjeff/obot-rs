#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputSafetyInputs {
    pub command_allows_output: bool,
    pub bus_allows_output: bool,
    pub driver_enabled: bool,
    pub driver_faulted: bool,
    pub controller_faulted: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputSafetyStatus {
    pub output_allowed: bool,
    pub command_blocked: bool,
    pub bus_blocked: bool,
    pub driver_not_enabled: bool,
    pub driver_fault_latched: bool,
    pub controller_faulted: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OutputSafety {
    driver_fault_latched: bool,
}

impl OutputSafety {
    pub const fn new() -> Self {
        Self {
            driver_fault_latched: false,
        }
    }

    pub fn update(&mut self, inputs: OutputSafetyInputs) -> OutputSafetyStatus {
        self.driver_fault_latched |= inputs.driver_faulted;

        let command_blocked = !inputs.command_allows_output;
        let bus_blocked = !inputs.bus_allows_output;
        let driver_not_enabled = !inputs.driver_enabled;
        let output_allowed = !command_blocked
            && !bus_blocked
            && !driver_not_enabled
            && !self.driver_fault_latched
            && !inputs.controller_faulted;

        OutputSafetyStatus {
            output_allowed,
            command_blocked,
            bus_blocked,
            driver_not_enabled,
            driver_fault_latched: self.driver_fault_latched,
            controller_faulted: inputs.controller_faulted,
        }
    }

    pub fn clear_latched_driver_fault(&mut self) {
        self.driver_fault_latched = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed_inputs() -> OutputSafetyInputs {
        OutputSafetyInputs {
            command_allows_output: true,
            bus_allows_output: true,
            driver_enabled: true,
            driver_faulted: false,
            controller_faulted: false,
        }
    }

    #[test]
    fn permits_output_only_when_every_gate_allows_it() {
        let mut safety = OutputSafety::new();
        assert!(safety.update(allowed_inputs()).output_allowed);

        let mut inputs = allowed_inputs();
        inputs.command_allows_output = false;
        assert_eq!(
            safety.update(inputs),
            OutputSafetyStatus {
                output_allowed: false,
                command_blocked: true,
                ..OutputSafetyStatus::default()
            }
        );

        let mut inputs = allowed_inputs();
        inputs.bus_allows_output = false;
        assert!(safety.update(inputs).bus_blocked);

        let mut inputs = allowed_inputs();
        inputs.driver_enabled = false;
        assert!(safety.update(inputs).driver_not_enabled);

        let mut inputs = allowed_inputs();
        inputs.controller_faulted = true;
        assert!(safety.update(inputs).controller_faulted);
    }

    #[test]
    fn latches_driver_fault_until_explicitly_cleared() {
        let mut safety = OutputSafety::new();
        let mut inputs = allowed_inputs();
        inputs.driver_faulted = true;

        let faulted = safety.update(inputs);
        assert!(!faulted.output_allowed);
        assert!(faulted.driver_fault_latched);

        let still_faulted = safety.update(allowed_inputs());
        assert!(!still_faulted.output_allowed);
        assert!(still_faulted.driver_fault_latched);

        safety.clear_latched_driver_fault();
        assert!(safety.update(allowed_inputs()).output_allowed);
    }
}
