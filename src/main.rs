use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
struct Config {
    latitude: Option<f64>,
    longitude: Option<f64>,
    verbose: bool,
    theme: Theme,
    units: Units,
    wind_units: WindUnits,
    temperature_label: bool,
    tooltip: bool,
    location_label: String,
    api_base: String,
    input_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Units {
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindUnits {
    Kmh,
    Mph,
    Ms,
    Kn,
}

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    apparent_temperature: f64,
    weather_code: i32,
    is_day: i32,
    wind_speed_10m: f64,
}

#[derive(Debug, Deserialize)]
struct IpLocation {
    latitude: Option<f64>,
    longitude: Option<f64>,
    city: Option<String>,
    success: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
enum LocationSource {
    ExplicitCoordinates,
    GeoClue,
    IpGeolocation,
}

fn main() {
    if let Err(error) = run() {
        let fallback = json!({
            "text": "N/A",
            "alt": "Weather unavailable",
            "tooltip": error,
            "class": ["waybar-weather", "weather-error"]
        });
        println!("{fallback}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_env_and_args()?;

    let mut detected_label = None;
    let weather = if let Some(input_file) = &config.input_file {
        if config.verbose {
            eprintln!(
                "waybar-weather: using input file {}; location lookup skipped",
                input_file.display()
            );
        }
        let body = fs::read_to_string(input_file).map_err(|err| err.to_string())?;
        serde_json::from_str::<WeatherResponse>(&body).map_err(|err| err.to_string())?
    } else {
        let (latitude, longitude, label, source) = resolve_location(&config)?;
        if config.verbose {
            eprintln!(
                "waybar-weather: location source={} latitude={latitude:.4} longitude={longitude:.4}",
                source.as_str()
            );
        }
        detected_label = label;
        fetch_weather(&config, latitude, longitude)?
    };

    let is_day = weather.current.is_day == 1;
    let condition_key = map_weather_code(weather.current.weather_code);
    let label = condition_label(condition_key);
    let has_icon = icon_filename(condition_key, is_day).is_some();
    let text = format_temperature(
        weather.current.temperature_2m,
        config.units,
        config.temperature_label,
    );

    let effective_location = if !config.location_label.is_empty() {
        config.location_label.clone()
    } else {
        detected_label.unwrap_or_default()
    };

    let mut tooltip_lines = Vec::new();
    if config.tooltip {
        if !effective_location.is_empty() {
            tooltip_lines.push(effective_location);
        }
        tooltip_lines.push(label.to_string());
        tooltip_lines.push(format!(
            "Temperature: {}",
            format_temperature(weather.current.temperature_2m, config.units, true)
        ));
        tooltip_lines.push(format!(
            "Feels like: {}",
            format_temperature(weather.current.apparent_temperature, config.units, true)
        ));
        tooltip_lines.push(format!(
            "Wind: {}",
            format_wind(weather.current.wind_speed_10m, config.wind_units)
        ));
    }

    let output = json!({
        "text": text,
        "alt": label,
        "tooltip": tooltip_lines.join("\n"),
        "class": [
            "waybar-weather",
            format!("weather-{condition_key}"),
            format!("weather-theme-{}", config.theme.as_str()),
            if is_day { "weather-day" } else { "weather-night" },
            if has_icon { "weather-has-icon" } else { "weather-missing-icon" }
        ],
        "percentage": weather.current.temperature_2m.round() as i64
    });

    println!("{output}");
    Ok(())
}

impl Config {
    fn from_env_and_args() -> Result<Self, String> {
        let mut args = HashMap::new();
        let raw_args: Vec<String> = env::args().skip(1).collect();
        let mut index = 0;

        while index < raw_args.len() {
            let arg = &raw_args[index];
            if !arg.starts_with("--") {
                index += 1;
                continue;
            }

            let trimmed = &arg[2..];
            if let Some((key, value)) = trimmed.split_once('=') {
                args.insert(key.replace('-', "_"), value.to_string());
                index += 1;
                continue;
            }

            let key = trimmed.replace('-', "_");
            if index + 1 < raw_args.len() && !raw_args[index + 1].starts_with("--") {
                args.insert(key, raw_args[index + 1].clone());
                index += 2;
            } else {
                args.insert(key, "true".to_string());
                index += 1;
            }
        }

        Ok(Self {
            latitude: get_setting(&args, "latitude", "WAYBAR_WEATHER_LATITUDE")
                .and_then(|value| value.parse::<f64>().ok()),
            longitude: get_setting(&args, "longitude", "WAYBAR_WEATHER_LONGITUDE")
                .and_then(|value| value.parse::<f64>().ok()),
            verbose: get_setting(&args, "verbose", "WAYBAR_WEATHER_VERBOSE")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(false),
            theme: Theme::parse(get_setting(&args, "theme", "WAYBAR_WEATHER_THEME").as_deref()),
            units: Units::parse(get_setting(&args, "units", "WAYBAR_WEATHER_UNITS").as_deref()),
            wind_units: WindUnits::parse(
                get_setting(&args, "wind_units", "WAYBAR_WEATHER_WIND_UNITS").as_deref(),
            ),
            temperature_label: get_setting(
                &args,
                "temperature_label",
                "WAYBAR_WEATHER_TEMPERATURE_LABEL",
            )
            .as_deref()
            .map(parse_bool)
            .unwrap_or(true),
            tooltip: get_setting(&args, "tooltip", "WAYBAR_WEATHER_TOOLTIP")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(true),
            location_label: get_setting(&args, "location_label", "WAYBAR_WEATHER_LOCATION_LABEL")
                .unwrap_or_default(),
            api_base: get_setting(&args, "api_base", "WAYBAR_WEATHER_API_BASE")
                .unwrap_or_else(|| "https://api.open-meteo.com/v1/forecast".to_string()),
            input_file: get_setting(&args, "input_file", "WAYBAR_WEATHER_INPUT_FILE")
                .map(PathBuf::from),
        })
    }
}

impl Theme {
    fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("dark").trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl Units {
    fn parse(value: Option<&str>) -> Self {
        match value
            .unwrap_or("celsius")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "fahrenheit" | "f" | "imperial" => Self::Fahrenheit,
            _ => Self::Celsius,
        }
    }

    fn as_api_value(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
        }
    }
}

impl WindUnits {
    fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("kmh").trim().to_ascii_lowercase().as_str() {
            "mph" | "mih" => Self::Mph,
            "ms" | "mps" | "m/s" => Self::Ms,
            "kn" | "knot" | "knots" => Self::Kn,
            _ => Self::Kmh,
        }
    }

    fn as_api_value(self) -> &'static str {
        match self {
            Self::Kmh => "kmh",
            Self::Mph => "mph",
            Self::Ms => "ms",
            Self::Kn => "kn",
        }
    }
}

impl LocationSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitCoordinates => "explicit-coordinates",
            Self::GeoClue => "geoclue",
            Self::IpGeolocation => "ip-geolocation",
        }
    }
}

fn get_setting(args: &HashMap<String, String>, arg_key: &str, env_key: &str) -> Option<String> {
    args.get(arg_key)
        .cloned()
        .or_else(|| env::var(env_key).ok().filter(|value| !value.is_empty()))
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn icon_filename(condition: &str, is_day: bool) -> Option<&'static str> {
    match condition {
        "clear" => Some(if is_day {
            "sunny.svg"
        } else {
            "clear_night.svg"
        }),
        "mostly_clear" => Some(if is_day {
            "mostly_sunny.svg"
        } else {
            "mostly_clear_night.svg"
        }),
        "partly_cloudy" => Some(if is_day {
            "partly_cloudy.svg"
        } else {
            "partly_cloudy_night.svg"
        }),
        "cloudy" => Some("cloudy.svg"),
        "fog" => Some("cloudy.svg"),
        "drizzle" => Some("drizzle.svg"),
        "light_rain" => Some("drizzle.svg"),
        "rain" => Some("heavy_rain.svg"),
        "heavy_rain" => Some("heavy_rain.svg"),
        "sleet_hail" => Some("sleet_hail.svg"),
        "mixed_rain_hail_sleet" => Some("wintry_mix.svg"),
        "light_snow" => Some("flurries.svg"),
        "snow" => Some("flurries.svg"),
        "heavy_snow" => Some("heavy_snow.svg"),
        "showers" => Some("heavy_rain.svg"),
        "scattered_showers" => Some("heavy_rain.svg"),
        "showers_snow" => Some("flurries.svg"),
        "scattered_snow_showers" => Some("flurries.svg"),
        "thunderstorms" => Some("strong_thunderstorms.svg"),
        "strong_thunderstorms" => Some("strong_thunderstorms.svg"),
        _ => None,
    }
}

fn resolve_location(config: &Config) -> Result<(f64, f64, Option<String>, LocationSource), String> {
    if let (Some(latitude), Some(longitude)) = (config.latitude, config.longitude) {
        return Ok((
            latitude,
            longitude,
            None,
            LocationSource::ExplicitCoordinates,
        ));
    }

    if let Some(location) = geoclue_location() {
        return Ok((location.0, location.1, location.2, LocationSource::GeoClue));
    }

    if let Some(location) = ip_location() {
        return Ok((
            location.0,
            location.1,
            location.2,
            LocationSource::IpGeolocation,
        ));
    }

    Err("Missing coordinates and auto-location failed. Set WAYBAR_WEATHER_LATITUDE and WAYBAR_WEATHER_LONGITUDE or enable GeoClue/network geolocation.".to_string())
}

fn geoclue_location() -> Option<(f64, f64, Option<String>)> {
    let create = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            "/org/freedesktop/GeoClue2/Manager",
            "--method",
            "org.freedesktop.GeoClue2.Manager.GetClient",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(create.stdout).ok()?;
    let client_path = extract_dbus_path(&stdout)?;

    let _ = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            &client_path,
            "--method",
            "org.freedesktop.DBus.Properties.Set",
            "org.freedesktop.GeoClue2.Client",
            "DesktopId",
            "<'waybar-weather'>",
        ])
        .output()
        .ok()?;

    let _ = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            &client_path,
            "--method",
            "org.freedesktop.DBus.Properties.Set",
            "org.freedesktop.GeoClue2.Client",
            "RequestedAccuracyLevel",
            "<uint32 4>",
        ])
        .output()
        .ok()?;

    let start = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            &client_path,
            "--method",
            "org.freedesktop.GeoClue2.Client.Start",
        ])
        .output()
        .ok()?;

    if !start.status.success() {
        return None;
    }

    let props = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            &client_path,
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.freedesktop.GeoClue2.Client",
            "Location",
        ])
        .output()
        .ok()?;

    let props_stdout = String::from_utf8(props.stdout).ok()?;
    let location_path = extract_dbus_path(&props_stdout)?;

    let location = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            &location_path,
            "--method",
            "org.freedesktop.DBus.Properties.GetAll",
            "org.freedesktop.GeoClue2.Location",
        ])
        .output()
        .ok()?;

    let location_stdout = String::from_utf8(location.stdout).ok()?;
    let latitude = extract_dbus_number(&location_stdout, "Latitude")?;
    let longitude = extract_dbus_number(&location_stdout, "Longitude")?;
    Some((latitude, longitude, None))
}

fn ip_location() -> Option<(f64, f64, Option<String>)> {
    let output = Command::new("curl")
        .args(["-fsSL", "https://ipwho.is/"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let parsed: IpLocation = serde_json::from_slice(&output.stdout).ok()?;
    if matches!(parsed.success, Some(false)) {
        return None;
    }

    Some((
        parsed.latitude?,
        parsed.longitude?,
        parsed.city.filter(|city| !city.is_empty()),
    ))
}

fn extract_dbus_path(value: &str) -> Option<String> {
    let start = value.find("objectpath '")? + "objectpath '".len();
    let end = value[start..].find('\'')? + start;
    Some(value[start..end].to_string())
}

fn extract_dbus_number(value: &str, key: &str) -> Option<f64> {
    let marker = format!("'{}': <", key);
    let start = value.find(&marker)? + marker.len();
    let end = value[start..].find('>')? + start;
    let raw = value[start..end].trim();
    let number = raw
        .trim_start_matches("double ")
        .trim_start_matches("uint32 ")
        .trim();
    number.parse::<f64>().ok()
}

fn fetch_weather(
    config: &Config,
    latitude: f64,
    longitude: f64,
) -> Result<WeatherResponse, String> {
    let url = format!(
        "{}?latitude={latitude}&longitude={longitude}&current=temperature_2m,apparent_temperature,weather_code,is_day,wind_speed_10m&timezone=auto&temperature_unit={}&wind_speed_unit={}",
        config.api_base,
        config.units.as_api_value(),
        config.wind_units.as_api_value()
    );

    let output = Command::new("curl")
        .args(["-fsSL", &url])
        .output()
        .map_err(|err| err.to_string())?;

    if !output.status.success() {
        return Err("Weather API request failed.".to_string());
    }

    serde_json::from_slice(&output.stdout).map_err(|err| err.to_string())
}

fn map_weather_code(code: i32) -> &'static str {
    match code {
        0 => "clear",
        1 => "mostly_clear",
        2 => "partly_cloudy",
        3 => "cloudy",
        45 | 48 => "fog",
        51 | 53 | 55 => "drizzle",
        56 | 57 => "sleet_hail",
        61 => "light_rain",
        63 => "rain",
        65 => "heavy_rain",
        66 | 67 => "mixed_rain_hail_sleet",
        71 => "light_snow",
        73 | 77 => "snow",
        75 => "heavy_snow",
        80 => "showers",
        81 => "scattered_showers",
        82 => "heavy_rain",
        85 => "showers_snow",
        86 => "scattered_snow_showers",
        95 => "thunderstorms",
        96 | 99 => "strong_thunderstorms",
        _ => "cloudy",
    }
}

fn condition_label(condition: &str) -> &'static str {
    match condition {
        "clear" => "Clear Sky",
        "mostly_clear" => "Mostly Clear",
        "partly_cloudy" => "Partly Cloudy",
        "cloudy" => "Cloudy",
        "fog" => "Fog",
        "drizzle" => "Drizzle",
        "sleet_hail" => "Sleet / Hail",
        "light_rain" => "Light Rain",
        "rain" => "Rain",
        "heavy_rain" => "Heavy Rain",
        "mixed_rain_hail_sleet" => "Mixed Rain / Hail / Sleet",
        "light_snow" => "Light Snow",
        "snow" => "Snow",
        "heavy_snow" => "Heavy Snow",
        "showers" => "Showers",
        "scattered_showers" => "Scattered Showers",
        "showers_snow" => "Snow Showers",
        "scattered_snow_showers" => "Scattered Snow Showers",
        "thunderstorms" => "Thunderstorms",
        "strong_thunderstorms" => "Strong Thunderstorms",
        _ => "Weather",
    }
}

fn format_temperature(value: f64, units: Units, include_label: bool) -> String {
    let rounded = value.round() as i64;
    if include_label {
        format!("{rounded}°{}", units.suffix())
    } else {
        format!("{rounded}°")
    }
}

fn format_wind(value: f64, units: WindUnits) -> String {
    let rounded = value.round() as i64;
    match units {
        WindUnits::Mph => format!("{rounded} mph"),
        WindUnits::Ms => format!("{rounded} m/s"),
        WindUnits::Kn => format!("{rounded} kn"),
        WindUnits::Kmh => format!("{rounded} km/h"),
    }
}
