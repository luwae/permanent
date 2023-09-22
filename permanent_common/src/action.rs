use crate::profiler::{Profile, Profiler};

pub trait Action {
    fn run(&self) -> Option<Vec<Profile>>;
}

pub struct ActionChain {
    actions: Vec<Box<dyn Action>>,
    profile_file: Option<String>,
}

impl ActionChain {
    pub fn new() -> Self {
        Self { actions: vec![], profile_file: None }
    }

    pub fn with_profiling(profile_file: &str) -> Self {
        Self { actions: vec![], profile_file: Some(profile_file.to_string()) }
    }

    pub fn append(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

}

impl Action for ActionChain {
    fn run(&self) -> Option<Vec<Profile>> {

        let mut profiler = Profiler::new();

        for action in &self.actions {
            match action.run() {
                None => break,
                Some(res) => profiler.register_vec(res),
            }
        }

        let profiles = profiler.get_profiles();
        if let Some(profile_file) = &self.profile_file {
            println!("== Write Profiling data");
            profiler.to_file(&profile_file[..]).expect("Could not write profiling data");
        }

        return Some(profiles);
    }
}
