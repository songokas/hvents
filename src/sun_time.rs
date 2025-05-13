use chrono::{DateTime, NaiveDate, Utc};
use sunrise::{Coordinates, SolarDay, SolarEvent};

pub fn sunrise(latitude: f64, longitude: f64, date: NaiveDate) -> DateTime<Utc> {
    let solar_day = SolarDay::new(
        Coordinates::new(latitude, longitude).expect("invalid coordinates"),
        date,
    );
    solar_day.event_time(SolarEvent::Sunrise)
}

pub fn sunset(latitude: f64, longitude: f64, date: NaiveDate) -> DateTime<Utc> {
    let solar_day = SolarDay::new(
        Coordinates::new(latitude, longitude).expect("invalid coordinates"),
        date,
    );
    solar_day.event_time(SolarEvent::Sunset)
}
