# Waybar Weather Module

Custom Waybar weather module implemented as a small Rust binary.

## Files

- `src/main.rs`: module runtime that emits Waybar JSON.
- `icons/`: static icon assets (`icons/light/*.svg`, `icons/dark/*.svg`).
- `waybar-weather.css`: static class-to-icon mapping.
- `examples/open-meteo-current.json`: fixture for offline testing.

## Build

```bash
cargo build --release
```

Binary path:

```bash
./target/release/waybar-weather
```

## Waybar Setup

Waybar `config.jsonc`:

```json
"custom/weather": {
  "return-type": "json",
  "exec": "/home/asif/.config/waybar/weather-module/waybar-weather",
  "interval": 900,
  "format": "{}",
  "tooltip": true
}
```

Waybar `style.css`:

```css
@import url("/home/asif/.config/waybar/weather-module/waybar-weather.css");
```

## Supported Configuration

Environment variables or CLI flags:

- `latitude` / `WAYBAR_WEATHER_LATITUDE`
- `longitude` / `WAYBAR_WEATHER_LONGITUDE`
- `theme` / `WAYBAR_WEATHER_THEME` (`dark` default)
- `units` / `WAYBAR_WEATHER_UNITS` (`celsius` default)
- `wind-units` / `WAYBAR_WEATHER_WIND_UNITS` (`kmh` default)
- `location-label` / `WAYBAR_WEATHER_LOCATION_LABEL`
- `temperature-label` / `WAYBAR_WEATHER_TEMPERATURE_LABEL` (`true` default)
- `tooltip` / `WAYBAR_WEATHER_TOOLTIP` (`true` default)
- `input-file` / `WAYBAR_WEATHER_INPUT_FILE`
- `verbose` / `WAYBAR_WEATHER_VERBOSE`

## Notes

- The binary emits classes only. CSS is static and should be edited manually when icon mapping changes.
- Auto-location fallback order is explicit coordinates, GeoClue, then IP geolocation (`ipwho.is`).
- Use `--verbose` to print location source diagnostics to `stderr`.
