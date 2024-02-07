use rand::Rng;
use serde::{Deserialize, Serialize};

pub struct Sensor(pub String);

impl Sensor {
    pub fn new_reading(&self) -> SensorReading {
        let mut rng = rand::thread_rng();

        // Generate a random number between 0 and 1.
        let random_percentage: f32 = rng.gen_range(0.0..1.0);

        let value = if random_percentage < 0.05 {
            // 5% chance: Value between 175 and 180
            rng.gen_range(175..=180)
        } else if random_percentage < 0.08 {
            // 3% chance: Value between 200 and 210
            rng.gen_range(200..=210)
        } else {
            // 92% chance: Value between 180 and 199
            rng.gen_range(180..=199)
        };

        SensorReading {
            id: self.0.clone(),
            value,
            timestamp: chrono::Utc::now()
        }
    }

    pub fn bad_reading(&self) -> SensorReading {
        let mut rng = rand::thread_rng();

        // Generate a random number between 0 and 1.
        let random_percentage: f32 = rng.gen_range(0.0..1.0);

        let value = if random_percentage < 0.15 {
            // 15% chance: Value between 175 and 180
            rng.gen_range(175..=180)
        } else if random_percentage < 0.20 {
            // 5% chance: Value between 200 and 210
            rng.gen_range(200..=210)
        } else {
            // 80% chance: Value between 180 and 199
            rng.gen_range(180..=199)
        };

        SensorReading {
            id: self.0.clone(),
            value,
            timestamp: chrono::Utc::now()
        }
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub id: String,
    pub value: u8,
    pub timestamp: chrono::DateTime<chrono::Utc>
}
