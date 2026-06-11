use super::sexp::{Sexp, child_list, list_children, maybe_list, numeric_at, string_at, tag};
use super::{Point, parse_at_point, parse_properties};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
struct PinGeometry {
    at: Point,
    name: Option<String>,
    hidden: bool,
}

#[derive(Debug, Default, Clone)]
pub(super) struct LibSymbolGeometry {
    common_pins: BTreeMap<String, PinGeometry>,
    unit_pins: BTreeMap<u32, BTreeMap<String, PinGeometry>>,
}

#[derive(Debug)]
pub(super) struct SymbolInstance {
    pub(super) refdes: String,
    pub(super) value: Option<String>,
    pub(super) lib: Option<String>,
    pub(super) part: Option<String>,
    pub(super) fields: BTreeMap<String, String>,
    pub(super) at: Point,
    pub(super) lib_id: String,
    pub(super) pins: BTreeMap<String, Point>,
    pub(super) is_power_symbol: bool,
}

pub(super) type PowerLabel = (Point, String);
pub(super) type ParsedSymbols = (Vec<SymbolInstance>, Vec<PowerLabel>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MirrorAxis {
    None,
    X,
    Y,
}

pub(super) fn parse_lib_symbol_pins(root: &[Sexp]) -> Result<BTreeMap<String, LibSymbolGeometry>> {
    let Some(lib_symbols) = child_list(root, "lib_symbols") else {
        bail!("Native KiCad schematic import requires root lib_symbols pin geometry.");
    };
    let mut raw_symbols = BTreeMap::new();
    for child in list_children(lib_symbols, "symbol") {
        let Some(lib_id) = string_at(child, 1) else {
            continue;
        };
        if raw_symbols.insert(lib_id.to_string(), child).is_some() {
            bail!("KiCad lib_symbols has duplicate top-level symbol {lib_id}.");
        }
    }
    let mut pins_by_lib = BTreeMap::new();
    for lib_id in raw_symbols.keys() {
        resolve_lib_symbol_geometry(lib_id, &raw_symbols, &mut pins_by_lib, &mut Vec::new())?;
    }
    Ok(pins_by_lib)
}

fn resolve_lib_symbol_geometry(
    lib_id: &str,
    raw_symbols: &BTreeMap<String, &[Sexp]>,
    pins_by_lib: &mut BTreeMap<String, LibSymbolGeometry>,
    stack: &mut Vec<String>,
) -> Result<LibSymbolGeometry> {
    if let Some(geometry) = pins_by_lib.get(lib_id) {
        return Ok(geometry.clone());
    }
    if stack.iter().any(|entry| entry == lib_id) {
        let mut cycle = stack.join(" -> ");
        if !cycle.is_empty() {
            cycle.push_str(" -> ");
        }
        cycle.push_str(lib_id);
        bail!("KiCad library symbol inheritance cycle detected: {cycle}.");
    }
    let Some(symbol) = raw_symbols.get(lib_id) else {
        bail!("KiCad library symbol {lib_id} is referenced but missing from lib_symbols.");
    };
    stack.push(lib_id.to_string());
    let geometry = if let Some(parent) = parse_lib_symbol_extends(symbol, lib_id)? {
        reject_extended_symbol_connectivity(symbol, lib_id)?;
        if !raw_symbols.contains_key(&parent) {
            bail!("KiCad library symbol {lib_id} extends missing base {parent}.");
        }
        resolve_lib_symbol_geometry(&parent, raw_symbols, pins_by_lib, stack)?
    } else {
        parse_lib_symbol_geometry(symbol, lib_id)?
    };
    stack.pop();
    pins_by_lib.insert(lib_id.to_string(), geometry.clone());
    Ok(geometry)
}

fn parse_lib_symbol_extends(list: &[Sexp], lib_id: &str) -> Result<Option<String>> {
    let Some(extends) = child_list(list, "extends") else {
        return Ok(None);
    };
    if extends.len() != 2 {
        bail!("KiCad library symbol {lib_id} has malformed extends token.");
    }
    let parent = string_at(extends, 1)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("KiCad library symbol {lib_id} has empty extends target."))?;
    Ok(Some(parent.to_string()))
}

fn reject_extended_symbol_connectivity(list: &[Sexp], lib_id: &str) -> Result<()> {
    for child in list.iter().skip(1).filter_map(maybe_list) {
        match tag(child) {
            Some("pin") => {
                bail!(
                    "KiCad library symbol {lib_id} extends another symbol and cannot declare pins."
                );
            }
            Some("symbol") => {
                bail!(
                    "KiCad library symbol {lib_id} extends another symbol and cannot declare unit symbols."
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_lib_symbol_geometry(list: &[Sexp], lib_id: &str) -> Result<LibSymbolGeometry> {
    let mut geometry = LibSymbolGeometry::default();
    for child in list.iter().skip(1).filter_map(maybe_list) {
        match tag(child) {
            Some("pin") => insert_pin_geometry(&mut geometry.common_pins, child, lib_id)?,
            Some("symbol") => {
                let Some(unit_id) = string_at(child, 1) else {
                    continue;
                };
                let direct_pin_count = child
                    .iter()
                    .skip(1)
                    .filter_map(maybe_list)
                    .filter(|entry| tag(entry) == Some("pin"))
                    .count();
                if direct_pin_count == 0 {
                    continue;
                }
                let unit = parse_lib_symbol_unit_id(lib_id, unit_id)?;
                let target = if unit == 0 {
                    &mut geometry.common_pins
                } else {
                    geometry.unit_pins.entry(unit).or_default()
                };
                for pin in child
                    .iter()
                    .skip(1)
                    .filter_map(maybe_list)
                    .filter(|entry| tag(entry) == Some("pin"))
                {
                    insert_pin_geometry(target, pin, unit_id)?;
                }
            }
            _ => {}
        }
    }
    Ok(geometry)
}

fn insert_pin_geometry(
    pins: &mut BTreeMap<String, PinGeometry>,
    pin: &[Sexp],
    context: &str,
) -> Result<()> {
    let electrical_type = string_at(pin, 1)
        .with_context(|| format!("KiCad library symbol {context} pin is missing electrical type."))?
        .to_string();
    let number = child_list(pin, "number")
        .and_then(|number| string_at(number, 1))
        .with_context(|| format!("KiCad library symbol {context} pin is missing a number."))?
        .to_string();
    let name = child_list(pin, "name")
        .and_then(|name| string_at(name, 1))
        .map(str::to_string);
    let at = child_list(pin, "at")
        .and_then(parse_at_point)
        .with_context(|| {
            format!("KiCad library symbol {context} pin {number} is missing coordinates.")
        })?;
    let hidden = pin
        .iter()
        .any(|entry| matches!(entry, Sexp::Atom(atom) if atom == "hide"));
    if hidden && electrical_type != "power_in" {
        bail!(
            "KiCad library symbol {context} pin {number} is hidden but has unsupported electrical type {electrical_type}."
        );
    }
    if hidden
        && name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        bail!("KiCad library symbol {context} hidden power pin {number} is missing a name.");
    }
    if pins
        .insert(number.clone(), PinGeometry { at, name, hidden })
        .is_some()
    {
        bail!("KiCad library symbol {context} has duplicate pin geometry for pin {number}.");
    }
    Ok(())
}

fn parse_lib_symbol_unit_id(parent: &str, unit_id: &str) -> Result<u32> {
    let (name_and_unit, _style) = unit_id.rsplit_once('_').with_context(|| {
        format!("KiCad library unit symbol {unit_id} does not match NAME_UNIT_STYLE.")
    })?;
    let (name, unit) = name_and_unit.rsplit_once('_').with_context(|| {
        format!("KiCad library unit symbol {unit_id} does not match NAME_UNIT_STYLE.")
    })?;
    if name != parent {
        bail!("KiCad library unit symbol {unit_id} does not belong to parent {parent}.");
    }
    unit.parse::<u32>().with_context(|| {
        format!("KiCad library unit symbol {unit_id} has a non-integer unit ordinal.")
    })
}

pub(super) fn parse_symbol_instances(
    root: &[Sexp],
    lib_pins: &BTreeMap<String, LibSymbolGeometry>,
) -> Result<ParsedSymbols> {
    let mut symbols = Vec::new();
    let mut power_labels = Vec::new();
    for symbol in list_children(root, "symbol") {
        let lib_id = child_list(symbol, "lib_id")
            .and_then(|list| string_at(list, 1))
            .with_context(|| "KiCad schematic symbol is missing lib_id.")?
            .to_string();
        let at_list = child_list(symbol, "at")
            .with_context(|| format!("KiCad schematic symbol {lib_id} is missing at."))?;
        let at = parse_at_point(at_list)
            .with_context(|| format!("KiCad schematic symbol {lib_id} has invalid at."))?;
        let rotation = parse_symbol_rotation(at_list, &lib_id)?;
        let mirror = parse_symbol_mirror(symbol, &lib_id)?;
        let unit = parse_symbol_unit(symbol, &lib_id)?;
        let properties = parse_properties(symbol);
        let refdes = properties
            .get("Reference")
            .filter(|value| !value.trim().is_empty())
            .with_context(|| format!("KiCad schematic symbol {lib_id} is missing Reference."))?
            .to_string();
        let value = properties.get("Value").cloned();
        let (lib, part) = split_lib_id(&lib_id);
        let is_power_symbol = refdes.starts_with("#PWR") || lib.as_deref() == Some("power");
        let on_board = parse_yes_no_token(symbol, "on_board", true, &refdes)?;
        if !is_power_symbol && !on_board {
            continue;
        }
        let Some(pin_geometry) = lib_pins.get(&lib_id) else {
            bail!(
                "KiCad schematic symbol {refdes} uses {lib_id}, but lib_symbols has no pin geometry for it."
            );
        };
        let pin_geometry = select_lib_symbol_pins(pin_geometry, unit, &refdes, &lib_id)?;
        let mut pins = BTreeMap::new();
        let mut explicit_pin_numbers = BTreeSet::new();
        for pin in list_children(symbol, "pin") {
            let number = string_at(pin, 1)
                .with_context(|| {
                    format!("KiCad schematic symbol {refdes} has a pin without a number.")
                })?
                .to_string();
            explicit_pin_numbers.insert(number.clone());
            let Some(geometry) = pin_geometry.get(&number) else {
                bail!(
                    "KiCad schematic symbol {refdes}.{number} has no matching lib_symbols pin geometry."
                );
            };
            let rotated = transform_pin_offset(geometry.at, mirror, rotation);
            pins.insert(
                number,
                Point {
                    x: at.x + rotated.x,
                    y: at.y + rotated.y,
                },
            );
        }
        for (number, geometry) in &pin_geometry {
            if !geometry.hidden || explicit_pin_numbers.contains(number) {
                continue;
            }
            let label = geometry
                .name
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .with_context(|| {
                    format!(
                        "KiCad schematic symbol {refdes}.{number} hidden power pin has no name."
                    )
                })?
                .to_string();
            let rotated = transform_pin_offset(geometry.at, mirror, rotation);
            let point = Point {
                x: at.x + rotated.x,
                y: at.y + rotated.y,
            };
            pins.insert(number.clone(), point);
            power_labels.push((point, label));
        }
        if pins.is_empty() {
            bail!("KiCad schematic symbol {refdes} has no instance pins.");
        }
        if is_power_symbol {
            if pins.len() != 1 {
                bail!("KiCad power symbol {refdes} must expose exactly one pin.");
            }
            let label = value
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .with_context(|| {
                    format!("KiCad power symbol {refdes} is missing a non-empty Value label.")
                })?
                .to_string();
            power_labels.push((*pins.values().next().unwrap(), label));
            continue;
        }
        let fields = properties
            .into_iter()
            .filter(|(name, _)| name != "Reference" && name != "Value")
            .collect();
        symbols.push(SymbolInstance {
            refdes,
            value,
            lib,
            part,
            fields,
            at,
            lib_id,
            pins,
            is_power_symbol,
        });
    }
    Ok((symbols, power_labels))
}

fn parse_yes_no_token(
    symbol: &[Sexp],
    token_name: &str,
    default: bool,
    refdes: &str,
) -> Result<bool> {
    let Some(list) = child_list(symbol, token_name) else {
        return Ok(default);
    };
    if list.len() != 2 {
        bail!("KiCad schematic symbol {refdes} has malformed {token_name} token.");
    }
    let value = string_at(list, 1).with_context(|| {
        format!("KiCad schematic symbol {refdes} has malformed {token_name} token.")
    })?;
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => bail!("KiCad schematic symbol {refdes} {token_name} must be yes or no, got {value}."),
    }
}

fn parse_symbol_unit(symbol: &[Sexp], lib_id: &str) -> Result<u32> {
    let Some(unit_list) = child_list(symbol, "unit") else {
        return Ok(1);
    };
    let unit = numeric_at(unit_list, 1)
        .with_context(|| format!("KiCad schematic symbol {lib_id} has malformed unit."))?;
    if !unit.is_finite() || unit.fract() != 0.0 || unit < 1.0 {
        bail!("KiCad schematic symbol {lib_id} unit must be a positive integer.");
    }
    Ok(unit as u32)
}

fn select_lib_symbol_pins<'a>(
    geometry: &'a LibSymbolGeometry,
    unit: u32,
    refdes: &str,
    lib_id: &str,
) -> Result<BTreeMap<String, &'a PinGeometry>> {
    let mut selected = geometry
        .common_pins
        .iter()
        .map(|(pin, geometry)| (pin.clone(), geometry))
        .collect::<BTreeMap<_, _>>();
    if !geometry.unit_pins.is_empty() {
        let Some(unit_pins) = geometry.unit_pins.get(&unit) else {
            let units = geometry.unit_pins.keys().copied().collect::<Vec<_>>();
            bail!(
                "KiCad schematic symbol {refdes} selects unit {unit}, but {lib_id} declares units {units:?}."
            );
        };
        for (pin, pin_geometry) in unit_pins {
            if selected.insert(pin.clone(), pin_geometry).is_some() {
                bail!(
                    "KiCad schematic symbol {refdes} unit {unit} duplicates common pin {pin} in {lib_id}."
                );
            }
        }
    }
    Ok(selected)
}

fn split_lib_id(lib_id: &str) -> (Option<String>, Option<String>) {
    lib_id
        .split_once(':')
        .map(|(lib, part)| (Some(lib.to_string()), Some(part.to_string())))
        .unwrap_or((None, Some(lib_id.to_string())))
}

fn parse_symbol_rotation(at_list: &[Sexp], lib_id: &str) -> Result<u16> {
    let Some(raw) = at_list.get(3) else {
        return Ok(0);
    };
    let raw = match raw {
        Sexp::Atom(value) | Sexp::Str(value) => value,
        Sexp::List(_) => bail!("KiCad schematic symbol {lib_id} has malformed rotation angle."),
    };
    let angle = raw.parse::<f64>().with_context(|| {
        format!("KiCad schematic symbol {lib_id} has malformed rotation angle.")
    })?;
    if !angle.is_finite() {
        bail!("KiCad schematic symbol {lib_id} has non-finite rotation angle.");
    }
    parse_cardinal_rotation(angle).with_context(|| {
        format!(
            "Native KiCad schematic import supports only cardinal symbol rotations for {lib_id}."
        )
    })
}

fn parse_symbol_mirror(symbol: &[Sexp], lib_id: &str) -> Result<MirrorAxis> {
    let Some(mirror) = child_list(symbol, "mirror") else {
        return Ok(MirrorAxis::None);
    };
    if mirror.len() != 2 {
        bail!("KiCad schematic symbol {lib_id} has malformed mirror token.");
    }
    let axis = string_at(mirror, 1)
        .with_context(|| format!("KiCad schematic symbol {lib_id} has malformed mirror token."))?;
    match axis.to_ascii_lowercase().as_str() {
        "x" => Ok(MirrorAxis::X),
        "y" => Ok(MirrorAxis::Y),
        _ => bail!("KiCad schematic symbol {lib_id} has unsupported mirror axis {axis}."),
    }
}

fn parse_cardinal_rotation(angle_degrees: f64) -> Result<u16> {
    if !angle_degrees.is_finite() {
        bail!("rotation {angle_degrees} is not finite")
    }
    let normalized = angle_degrees.rem_euclid(360.0);
    for candidate in [0_u16, 90, 180, 270] {
        if (normalized - f64::from(candidate)).abs() < 1e-9 {
            return Ok(candidate);
        }
    }
    bail!("rotation {angle_degrees} is not a cardinal angle")
}

fn transform_pin_offset(point: Point, mirror: MirrorAxis, rotation: u16) -> Point {
    rotate_point(mirror_point(point, mirror), rotation)
}

fn mirror_point(point: Point, mirror: MirrorAxis) -> Point {
    match mirror {
        MirrorAxis::None => point,
        MirrorAxis::X => Point {
            x: point.x,
            y: -point.y,
        },
        MirrorAxis::Y => Point {
            x: -point.x,
            y: point.y,
        },
    }
}

fn rotate_point(point: Point, rotation: u16) -> Point {
    match rotation {
        0 => point,
        90 => Point {
            x: -point.y,
            y: point.x,
        },
        180 => Point {
            x: -point.x,
            y: -point.y,
        },
        270 => Point {
            x: point.y,
            y: -point.x,
        },
        _ => unreachable!("rotation is validated by parse_cardinal_rotation"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MirrorAxis, Point, Sexp, mirror_point, parse_cardinal_rotation, parse_symbol_mirror,
        parse_symbol_rotation, rotate_point, transform_pin_offset,
    };

    #[test]
    fn rotates_cardinal_pin_offsets() {
        let point = Point {
            x: 10_000_000,
            y: -20_000_000,
        };
        assert_eq!(rotate_point(point, 0), point);
        assert_eq!(
            rotate_point(point, 90),
            Point {
                x: 20_000_000,
                y: 10_000_000
            }
        );
        assert_eq!(
            rotate_point(point, 180),
            Point {
                x: -10_000_000,
                y: 20_000_000
            }
        );
        assert_eq!(
            rotate_point(point, 270),
            Point {
                x: -20_000_000,
                y: -10_000_000
            }
        );
        assert_eq!(parse_cardinal_rotation(-90.0).unwrap(), 270);
        assert_eq!(parse_cardinal_rotation(360.0).unwrap(), 0);
        assert_eq!(parse_cardinal_rotation(450.0).unwrap(), 90);
        assert!(parse_cardinal_rotation(45.0).is_err());
        assert!(parse_cardinal_rotation(89.999).is_err());
        assert!(parse_cardinal_rotation(90.1).is_err());
        assert!(parse_cardinal_rotation(450.1).is_err());
    }

    #[test]
    fn mirrors_pin_offsets_before_rotation() {
        let point = Point {
            x: 10_000_000,
            y: -20_000_000,
        };
        assert_eq!(mirror_point(point, MirrorAxis::None), point);
        assert_eq!(
            mirror_point(point, MirrorAxis::X),
            Point {
                x: 10_000_000,
                y: 20_000_000
            }
        );
        assert_eq!(
            mirror_point(point, MirrorAxis::Y),
            Point {
                x: -10_000_000,
                y: -20_000_000
            }
        );
        assert_eq!(
            transform_pin_offset(point, MirrorAxis::X, 90),
            Point {
                x: -20_000_000,
                y: 10_000_000
            }
        );
    }

    #[test]
    fn rejects_malformed_symbol_rotation() {
        let malformed = vec![
            Sexp::Atom("at".to_string()),
            Sexp::Atom("0".to_string()),
            Sexp::Atom("0".to_string()),
            Sexp::Atom("bad".to_string()),
        ];
        assert!(parse_symbol_rotation(&malformed, "Device:R").is_err());
        let non_finite = vec![
            Sexp::Atom("at".to_string()),
            Sexp::Atom("0".to_string()),
            Sexp::Atom("0".to_string()),
            Sexp::Atom("NaN".to_string()),
        ];
        assert!(parse_symbol_rotation(&non_finite, "Device:R").is_err());
    }

    #[test]
    fn parses_and_rejects_symbol_mirror_tokens() {
        let mirrored_x = vec![
            Sexp::Atom("symbol".to_string()),
            Sexp::List(vec![
                Sexp::Atom("mirror".to_string()),
                Sexp::Atom("x".to_string()),
            ]),
        ];
        assert_eq!(
            parse_symbol_mirror(&mirrored_x, "Device:R").unwrap(),
            MirrorAxis::X
        );
        let mirrored_y = vec![
            Sexp::Atom("symbol".to_string()),
            Sexp::List(vec![
                Sexp::Atom("mirror".to_string()),
                Sexp::Atom("Y".to_string()),
            ]),
        ];
        assert_eq!(
            parse_symbol_mirror(&mirrored_y, "Device:R").unwrap(),
            MirrorAxis::Y
        );
        let unsupported = vec![
            Sexp::Atom("symbol".to_string()),
            Sexp::List(vec![
                Sexp::Atom("mirror".to_string()),
                Sexp::Atom("z".to_string()),
            ]),
        ];
        assert!(parse_symbol_mirror(&unsupported, "Device:R").is_err());
        let malformed = vec![
            Sexp::Atom("symbol".to_string()),
            Sexp::List(vec![
                Sexp::Atom("mirror".to_string()),
                Sexp::Atom("x".to_string()),
                Sexp::Atom("extra".to_string()),
            ]),
        ];
        assert!(parse_symbol_mirror(&malformed, "Device:R").is_err());
    }
}
