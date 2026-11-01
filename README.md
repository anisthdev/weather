# weather

Weather from Open-Meteo as JSON, from a small Rust binary: a Waybar module
by default, and with `--forecast` everything a weather card needs (the
current conditions, the next hours and days, air quality) in one document,
as quickbar's weather card reads it.

## Files

- `src/main.rs`: the binary.
- `icons/`: static icon assets (`icons/light/*.svg`, `icons/dark/*.svg`).
- `waybar-weather.css`: static class-to-icon mapping, for Waybar.
- `examples/open-meteo-current.json`: fixture for offline testing.

## Build and install

```bash
makepkg -si            # installs /usr/bin/weather
cargo build --release  # or just build: ./target/release/weather
```

## Use

```bash
weather                # Waybar's custom module JSON (text, tooltip, class)
weather --forecast     # the weather card's JSON
```

## Waybar Setup

Waybar `config.jsonc`:

```json
"custom/weather": {
  "return-type": "json",
  "exec": "weather",
  "interval": 900,
  "format": "{}",
  "tooltip": true
}
```

Waybar `style.css` imports `waybar-weather.css` (with the icons beside it)
for the class-to-icon mapping.

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
