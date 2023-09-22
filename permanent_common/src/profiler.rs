use std::time::{SystemTime, Duration, SystemTimeError};
use std::iter::Sum;
use std::fs::File;
use std::io::BufWriter;
use std::io;
use std::io::Write;

#[derive(Clone)]
pub struct Profiler {
    profiles: Vec<Profile>,
}

impl Profiler {
    pub fn new() -> Self {
        Self { profiles: vec!() }
    }

    pub fn from_profiles(profiles: Vec<Profile>) -> Self {
        Self { profiles     }
    }

    pub fn to_file(&self, filename: &str) -> Result<(), io::Error> {

        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);

        for profile in &self.profiles {
            writer.write_fmt(format_args!("{}\n", profile))?;
        }

        writer.flush()?;
        Ok(())

    }

    pub fn register(&mut self, profile: Profile) {
        self.profiles.push(profile);
    }

    pub fn register_vec(&mut self, profiles: Vec<Profile>) {
        for profile in profiles {
            self.profiles.push(profile);
        }
    }

    pub fn get_profiles(&self) -> Vec<Profile> {
        self.profiles.clone()
    }


}

#[derive(Clone)]
pub struct Profile {
    name: String,
    measurements: Vec<Measurement>,
    logs: Vec<Box<String>>,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), measurements: vec!(), logs: vec!() }
    }

    pub fn from_measurement(name: &str, measurement: Measurement) -> Self {
        Self { name: name.to_string(), measurements: vec!(measurement), logs: vec!() }
    }
    
    pub fn measure(&mut self, measurement: Measurement) {
        self.measurements.push(measurement);
    }

    pub fn measure_vec(&mut self, measurements: Vec<Measurement>) {
        for meas in measurements {
            self.measure(meas);
        }
    }

    pub fn get_measurements(&self) -> Vec<Measurement> {
        return self.measurements.clone();
    }

    pub fn log(&mut self, text: &str) {
        self.logs.push(Box::new(text.to_string()));
    }

    pub fn durations(&self) -> Vec<Duration> {
        let mut durations = vec!();

        for meas in &self.measurements {
            match meas.duration() {
                Ok(d) => durations.push(d),
                Err(_) => {},
            }
        }

        return durations
    }

    pub fn average_duration(&self) -> Duration {
        let durations = self.durations();
        let dur = Duration::sum(durations.iter());
        return if durations.len() == 0 {
            Duration::new(0, 0)
        }  else { 
            dur / durations.len() as u32
        }
    }

}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?} {:?}\n{:?}", self.name, self.average_duration(), self.durations(), self.logs)
    }
}

#[derive(Clone)]
pub struct Measurement {
    start: SystemTime,
    end: SystemTime,
}

impl Measurement {
    pub fn new(start: SystemTime, end: SystemTime) -> Self {
        Self { start, end }
    }
    pub fn duration(&self) -> Result<Duration, SystemTimeError> {
        self.end.duration_since(self.start)
    }
}
