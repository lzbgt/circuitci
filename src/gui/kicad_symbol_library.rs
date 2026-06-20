use super::sketch::SketchNodeStyle;
use super::sketch_symbols::SketchSymbolKind;
use crate::importers::kicad_sch::sexp::{
    Sexp, as_list, child_list, list_children, numeric_at, parse_sexp_document, string_at, tag,
};
use eframe::egui;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq)]
struct KiCadPoint {
    x: f32,
    y: f32,
}

impl KiCadPoint {
    fn new(x: f64, y: f64) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum KiCadSymbolPrimitive {
    Polyline(Vec<KiCadPoint>),
    Rectangle {
        start: KiCadPoint,
        end: KiCadPoint,
    },
    Circle {
        center: KiCadPoint,
        radius: f32,
    },
    Arc {
        start: KiCadPoint,
        mid: KiCadPoint,
        end: KiCadPoint,
    },
    Text {
        text: String,
        at: KiCadPoint,
    },
    PinLine {
        pin: Option<String>,
        start: KiCadPoint,
        end: KiCadPoint,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct KiCadSymbolDrawing {
    primitives: Vec<KiCadSymbolPrimitive>,
    min: KiCadPoint,
    max: KiCadPoint,
}

#[derive(Debug, Clone, Copy)]
struct KiCadSymbolSpec {
    key: &'static str,
    library: &'static str,
    symbol: &'static str,
    rotation_offset_deg: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KiCadSymbolCatalogEntry {
    pub(super) id: String,
    pub(super) library: String,
    pub(super) name: String,
    pub(super) source: String,
    pub(super) pins: Vec<KiCadSymbolPin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KiCadSymbolPin {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) electrical_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct KiCadSymbolPinAnchor {
    pub(super) pin: String,
    pub(super) pos: egui::Pos2,
    pub(super) label_pos: egui::Pos2,
    pub(super) label_align: egui::Align2,
}

#[derive(Clone, Copy)]
struct KiCadDrawContext<'a> {
    painter: &'a egui::Painter,
    drawing: &'a KiCadSymbolDrawing,
    rect: egui::Rect,
    style: SketchNodeStyle,
    rotation_offset_deg: i32,
    stroke: egui::Stroke,
    color: egui::Color32,
}

const DEFAULT_SYMBOL_SPECS: &[KiCadSymbolSpec] = &[
    KiCadSymbolSpec {
        key: "Device:R",
        library: "Device",
        symbol: "R",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:C",
        library: "Device",
        symbol: "C",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:L",
        library: "Device",
        symbol: "L",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:D",
        library: "Device",
        symbol: "D",
        rotation_offset_deg: 0,
    },
    KiCadSymbolSpec {
        key: "Simulation_SPICE:VDC",
        library: "Simulation_SPICE",
        symbol: "VDC",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:Voltmeter_DC",
        library: "Device",
        symbol: "Voltmeter_DC",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:Ammeter_DC",
        library: "Device",
        symbol: "Ammeter_DC",
        rotation_offset_deg: 90,
    },
    KiCadSymbolSpec {
        key: "Device:Oscilloscope",
        library: "Device",
        symbol: "Oscilloscope",
        rotation_offset_deg: 0,
    },
];

static DEFAULT_KICAD_SYMBOLS: OnceLock<BTreeMap<&'static str, KiCadSymbolDrawing>> =
    OnceLock::new();
static INSTALLED_KICAD_SYMBOL_CATALOG: OnceLock<Vec<KiCadSymbolCatalogEntry>> = OnceLock::new();
static KICAD_SYMBOL_DRAWINGS: OnceLock<Mutex<BTreeMap<String, Option<KiCadSymbolDrawing>>>> =
    OnceLock::new();

pub(super) fn draw_kicad_default_symbol(
    painter: &egui::Painter,
    kind: SketchSymbolKind,
    rect: egui::Rect,
    style: SketchNodeStyle,
    stroke: egui::Stroke,
    color: egui::Color32,
) -> bool {
    let Some(spec) = symbol_spec_for_kind(kind) else {
        return false;
    };
    let Some(drawing) = default_symbol_cache().get(spec.key) else {
        return false;
    };
    drawing.draw(
        painter,
        rect,
        style,
        spec.rotation_offset_deg,
        stroke,
        color,
    );
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KiCadProbeSymbolKind {
    Voltage,
    Current,
    Power,
}

pub(super) fn draw_kicad_probe_symbol(
    painter: &egui::Painter,
    kind: KiCadProbeSymbolKind,
    rect: egui::Rect,
    stroke: egui::Stroke,
    color: egui::Color32,
) -> bool {
    let Some(spec) = symbol_spec_for_probe(kind) else {
        return false;
    };
    let Some(drawing) = default_symbol_cache().get(spec.key) else {
        return false;
    };
    drawing.draw(
        painter,
        rect,
        SketchNodeStyle::default(),
        spec.rotation_offset_deg,
        stroke,
        color,
    );
    true
}

pub(super) fn draw_kicad_symbol_by_id(
    painter: &egui::Painter,
    symbol_id: &str,
    rect: egui::Rect,
    style: SketchNodeStyle,
    stroke: egui::Stroke,
    color: egui::Color32,
) -> bool {
    let Some(drawing) = cached_kicad_symbol_drawing(symbol_id) else {
        return false;
    };
    drawing.draw(painter, rect, style, 0, stroke, color);
    true
}

pub(super) fn kicad_symbol_pin_anchors(
    symbol_id: &str,
    rect: egui::Rect,
    style: SketchNodeStyle,
) -> Vec<KiCadSymbolPinAnchor> {
    cached_kicad_symbol_drawing(symbol_id)
        .map(|drawing| drawing.pin_anchors(rect.shrink2(egui::vec2(3.0, 3.0)), style, 0))
        .unwrap_or_default()
}

pub(super) fn kicad_default_symbol_pin_anchors(
    kind: SketchSymbolKind,
    rect: egui::Rect,
    style: SketchNodeStyle,
) -> Vec<KiCadSymbolPinAnchor> {
    let Some(spec) = symbol_spec_for_kind(kind) else {
        return Vec::new();
    };
    default_symbol_cache()
        .get(spec.key)
        .map(|drawing| {
            drawing.pin_anchors(
                rect.shrink2(egui::vec2(3.0, 3.0)),
                style,
                spec.rotation_offset_deg,
            )
        })
        .unwrap_or_default()
}

pub(super) fn installed_kicad_symbol_catalog() -> &'static [KiCadSymbolCatalogEntry] {
    INSTALLED_KICAD_SYMBOL_CATALOG.get_or_init(load_installed_kicad_symbol_catalog)
}

pub(super) fn kicad_symbol_catalog(
    imported_files: &[String],
) -> (Vec<KiCadSymbolCatalogEntry>, Vec<String>) {
    let mut entries = installed_kicad_symbol_catalog().to_vec();
    let mut diagnostics = Vec::new();
    for path in imported_files {
        match parse_kicad_symbol_catalog_file(Path::new(path)) {
            Ok(mut imported) => entries.append(&mut imported),
            Err(error) => diagnostics.push(format!("{path}: {error}")),
        }
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries.dedup_by(|left, right| left.id == right.id);
    (entries, diagnostics)
}

pub(super) fn import_kicad_symbol_file(
    path: &Path,
) -> anyhow::Result<Vec<KiCadSymbolCatalogEntry>> {
    let entries = parse_kicad_symbol_catalog_file(path)?;
    let cached = cache_kicad_symbol_drawings_from_file(path)?;
    if cached == 0 {
        anyhow::bail!("KiCad symbol file contains no drawable top-level symbols.");
    }
    Ok(entries)
}

fn symbol_spec_for_kind(kind: SketchSymbolKind) -> Option<KiCadSymbolSpec> {
    match kind {
        SketchSymbolKind::Resistor => spec_by_key("Device:R"),
        SketchSymbolKind::Capacitor => spec_by_key("Device:C"),
        SketchSymbolKind::Inductor => spec_by_key("Device:L"),
        SketchSymbolKind::Diode => spec_by_key("Device:D"),
        SketchSymbolKind::Source => spec_by_key("Simulation_SPICE:VDC"),
        _ => None,
    }
}

fn symbol_spec_for_probe(kind: KiCadProbeSymbolKind) -> Option<KiCadSymbolSpec> {
    match kind {
        KiCadProbeSymbolKind::Voltage => spec_by_key("Device:Voltmeter_DC"),
        KiCadProbeSymbolKind::Current => spec_by_key("Device:Ammeter_DC"),
        KiCadProbeSymbolKind::Power => spec_by_key("Device:Oscilloscope"),
    }
}

fn spec_by_key(key: &str) -> Option<KiCadSymbolSpec> {
    DEFAULT_SYMBOL_SPECS
        .iter()
        .copied()
        .find(|spec| spec.key == key)
}

fn default_symbol_cache() -> &'static BTreeMap<&'static str, KiCadSymbolDrawing> {
    DEFAULT_KICAD_SYMBOLS.get_or_init(load_default_symbol_cache)
}

fn load_default_symbol_cache() -> BTreeMap<&'static str, KiCadSymbolDrawing> {
    let mut by_library: BTreeMap<&str, Vec<KiCadSymbolSpec>> = BTreeMap::new();
    for spec in DEFAULT_SYMBOL_SPECS {
        by_library.entry(spec.library).or_default().push(*spec);
    }

    let mut loaded = BTreeMap::new();
    for root in installed_kicad_symbol_library_paths() {
        for (library, specs) in &by_library {
            let path = root.join(format!("{library}.kicad_sym"));
            if !path.exists() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for spec in specs {
                if loaded.contains_key(spec.key) {
                    continue;
                }
                if let Some(drawing) = parse_kicad_symbol_drawing(&text, spec.symbol) {
                    loaded.insert(spec.key, drawing);
                }
            }
        }
    }
    loaded
}

fn load_installed_kicad_symbol_catalog() -> Vec<KiCadSymbolCatalogEntry> {
    let mut entries = Vec::new();
    for root in installed_kicad_symbol_library_paths() {
        let Ok(read_dir) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("kicad_sym") {
                continue;
            }
            if let Ok(mut catalog) = parse_kicad_symbol_catalog_file(&path) {
                entries.append(&mut catalog);
            }
        }
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries.dedup_by(|left, right| left.id == right.id);
    entries
}

pub(super) fn installed_kicad_symbol_library_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for var in [
        "KICAD_SYMBOL_DIR",
        "KICAD9_SYMBOL_DIR",
        "KICAD8_SYMBOL_DIR",
        "KICAD7_SYMBOL_DIR",
        "KICAD6_SYMBOL_DIR",
    ] {
        if let Some(path) = std::env::var_os(var).map(PathBuf::from) {
            push_unique_existing_dir(&mut paths, path);
        }
    }
    for path in [
        "/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols",
        "/usr/share/kicad/symbols",
        "/usr/local/share/kicad/symbols",
        "/opt/homebrew/share/kicad/symbols",
        "/opt/kicad/share/kicad/symbols",
    ] {
        push_unique_existing_dir(&mut paths, PathBuf::from(path));
    }
    paths
}

fn push_unique_existing_dir(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_dir() && !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn parse_kicad_symbol_catalog_file(path: &Path) -> anyhow::Result<Vec<KiCadSymbolCatalogEntry>> {
    let library = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("KiCad symbol file name is not valid UTF-8."))?
        .to_string();
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read KiCad symbol file: {error}"))?;
    parse_kicad_symbol_catalog(&text, &library, &path.display().to_string())
}

pub(super) fn parse_kicad_symbol_catalog(
    text: &str,
    library: &str,
    source: &str,
) -> anyhow::Result<Vec<KiCadSymbolCatalogEntry>> {
    let sexp = parse_sexp_document(text)?;
    let root =
        as_list(&sexp).ok_or_else(|| anyhow::anyhow!("KiCad symbol file root must be a list."))?;
    let mut entries = Vec::new();
    for symbol in list_children(root, "symbol") {
        let Some(name) = string_at(symbol, 1).filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let mut pins = Vec::new();
        collect_symbol_pins(symbol, &mut pins);
        pins.sort_by(|left, right| left.id.cmp(&right.id));
        pins.dedup_by(|left, right| left.id == right.id);
        entries.push(KiCadSymbolCatalogEntry {
            id: format!("{library}:{name}"),
            library: library.to_string(),
            name: name.to_string(),
            source: source.to_string(),
            pins,
        });
    }
    if entries.is_empty() {
        anyhow::bail!("KiCad symbol file contains no top-level symbols.");
    }
    Ok(entries)
}

fn collect_symbol_pins(list: &[Sexp], pins: &mut Vec<KiCadSymbolPin>) {
    for child in list
        .iter()
        .skip(1)
        .filter_map(crate::importers::kicad_sch::sexp::maybe_list)
    {
        match tag(child) {
            Some("pin") => {
                let Some(id) = child_list(child, "number")
                    .and_then(|number| string_at(number, 1))
                    .or_else(|| child_list(child, "name").and_then(|name| string_at(name, 1)))
                    .filter(|value| !value.trim().is_empty())
                else {
                    continue;
                };
                let name = child_list(child, "name")
                    .and_then(|name| string_at(name, 1))
                    .unwrap_or("")
                    .to_string();
                pins.push(KiCadSymbolPin {
                    id: id.to_string(),
                    name,
                    electrical_type: string_at(child, 1).unwrap_or("passive").to_string(),
                });
            }
            Some("symbol") => collect_symbol_pins(child, pins),
            _ => {}
        }
    }
}

pub(super) fn parse_kicad_symbol_drawing(
    text: &str,
    symbol_name: &str,
) -> Option<KiCadSymbolDrawing> {
    let sexp = parse_sexp_document(text).ok()?;
    let root = as_list(&sexp)?;
    let symbol =
        list_children(root, "symbol").find(|symbol| string_at(symbol, 1) == Some(symbol_name))?;
    parse_kicad_symbol_drawing_from_list(symbol)
}

fn parse_kicad_symbol_drawing_from_list(symbol: &[Sexp]) -> Option<KiCadSymbolDrawing> {
    let mut primitives = Vec::new();
    collect_symbol_primitives(symbol, &mut primitives);
    KiCadSymbolDrawing::from_primitives(primitives)
}

fn cached_kicad_symbol_drawing(symbol_id: &str) -> Option<KiCadSymbolDrawing> {
    if let Some(cached) = kicad_symbol_drawing_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(symbol_id).cloned())
    {
        return cached;
    }
    let loaded = load_installed_kicad_symbol_drawing(symbol_id);
    if let Ok(mut cache) = kicad_symbol_drawing_cache().lock() {
        cache.insert(symbol_id.to_string(), loaded.clone());
    }
    loaded
}

fn kicad_symbol_drawing_cache() -> &'static Mutex<BTreeMap<String, Option<KiCadSymbolDrawing>>> {
    KICAD_SYMBOL_DRAWINGS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn load_installed_kicad_symbol_drawing(symbol_id: &str) -> Option<KiCadSymbolDrawing> {
    let (library, symbol) = symbol_id.split_once(':')?;
    for root in installed_kicad_symbol_library_paths() {
        let path = root.join(format!("{library}.kicad_sym"));
        if !path.exists() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if let Some(drawing) = parse_kicad_symbol_drawing(&text, symbol) {
            return Some(drawing);
        }
    }
    None
}

fn cache_kicad_symbol_drawings_from_file(path: &Path) -> anyhow::Result<usize> {
    let library = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("KiCad symbol file name is not valid UTF-8."))?
        .to_string();
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read KiCad symbol file: {error}"))?;
    let sexp = parse_sexp_document(&text)?;
    let root =
        as_list(&sexp).ok_or_else(|| anyhow::anyhow!("KiCad symbol file root must be a list."))?;
    let mut count = 0;
    let mut cache = kicad_symbol_drawing_cache()
        .lock()
        .map_err(|_| anyhow::anyhow!("KiCad symbol drawing cache is unavailable."))?;
    for symbol in list_children(root, "symbol") {
        let Some(name) = string_at(symbol, 1).filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let id = format!("{library}:{name}");
        let drawing = parse_kicad_symbol_drawing_from_list(symbol);
        if drawing.is_some() {
            count += 1;
        }
        cache.insert(id, drawing);
    }
    Ok(count)
}

fn collect_symbol_primitives(list: &[Sexp], primitives: &mut Vec<KiCadSymbolPrimitive>) {
    for child in list
        .iter()
        .skip(1)
        .filter_map(crate::importers::kicad_sch::sexp::maybe_list)
    {
        match tag(child) {
            Some("polyline") => {
                if let Some(points) = parse_points(child) {
                    primitives.push(KiCadSymbolPrimitive::Polyline(points));
                }
            }
            Some("rectangle") => {
                if let (Some(start), Some(end)) = (
                    child_list(child, "start").and_then(parse_xy_pair),
                    child_list(child, "end").and_then(parse_xy_pair),
                ) {
                    primitives.push(KiCadSymbolPrimitive::Rectangle { start, end });
                }
            }
            Some("circle") => {
                if let (Some(center), Some(radius)) = (
                    child_list(child, "center").and_then(parse_xy_pair),
                    child_list(child, "radius").and_then(|radius| numeric_at(radius, 1)),
                ) {
                    primitives.push(KiCadSymbolPrimitive::Circle {
                        center,
                        radius: radius as f32,
                    });
                }
            }
            Some("arc") => {
                if let (Some(start), Some(mid), Some(end)) = (
                    child_list(child, "start").and_then(parse_xy_pair),
                    child_list(child, "mid").and_then(parse_xy_pair),
                    child_list(child, "end").and_then(parse_xy_pair),
                ) {
                    primitives.push(KiCadSymbolPrimitive::Arc { start, mid, end });
                }
            }
            Some("text") => {
                if let (Some(text), Some(at)) = (
                    string_at(child, 1).map(str::to_string),
                    child_list(child, "at").and_then(parse_xy_pair),
                ) {
                    primitives.push(KiCadSymbolPrimitive::Text { text, at });
                }
            }
            Some("pin") => {
                if let Some((start, end)) = parse_pin_line(child) {
                    primitives.push(KiCadSymbolPrimitive::PinLine {
                        pin: parse_pin_id(child),
                        start,
                        end,
                    });
                }
            }
            Some("symbol") => collect_symbol_primitives(child, primitives),
            _ => {}
        }
    }
}

fn parse_points(list: &[Sexp]) -> Option<Vec<KiCadPoint>> {
    let points = child_list(list, "pts")?;
    let parsed: Vec<_> = list_children(points, "xy")
        .filter_map(parse_xy_pair)
        .collect();
    (parsed.len() >= 2).then_some(parsed)
}

fn parse_xy_pair(list: &[Sexp]) -> Option<KiCadPoint> {
    Some(KiCadPoint::new(numeric_at(list, 1)?, numeric_at(list, 2)?))
}

fn parse_pin_id(pin: &[Sexp]) -> Option<String> {
    child_list(pin, "number")
        .and_then(|number| string_at(number, 1))
        .or_else(|| child_list(pin, "name").and_then(|name| string_at(name, 1)))
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn parse_pin_line(pin: &[Sexp]) -> Option<(KiCadPoint, KiCadPoint)> {
    let at = child_list(pin, "at")?;
    let start = KiCadPoint::new(numeric_at(at, 1)?, numeric_at(at, 2)?);
    let angle = numeric_at(at, 3)? as f32;
    let length = child_list(pin, "length")
        .and_then(|length| numeric_at(length, 1))
        .unwrap_or(0.0) as f32;
    let radians = angle.to_radians();
    let end = KiCadPoint {
        x: start.x + radians.cos() * length,
        y: start.y + radians.sin() * length,
    };
    Some((start, end))
}

impl KiCadSymbolDrawing {
    fn from_primitives(primitives: Vec<KiCadSymbolPrimitive>) -> Option<Self> {
        if primitives.is_empty() {
            return None;
        }
        let mut points = Vec::new();
        for primitive in &primitives {
            primitive.collect_points(&mut points);
        }
        let first = *points.first()?;
        let mut min = first;
        let mut max = first;
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
        if (max.x - min.x).abs() < f32::EPSILON || (max.y - min.y).abs() < f32::EPSILON {
            return None;
        }
        Some(Self {
            primitives,
            min,
            max,
        })
    }

    fn draw(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        style: SketchNodeStyle,
        rotation_offset_deg: i32,
        stroke: egui::Stroke,
        color: egui::Color32,
    ) {
        let target = rect.shrink2(egui::vec2(3.0, 3.0));
        let context = KiCadDrawContext {
            painter,
            drawing: self,
            rect: target,
            style,
            rotation_offset_deg,
            stroke,
            color,
        };
        for primitive in &self.primitives {
            primitive.draw(&context);
        }
    }

    fn pin_anchors(
        &self,
        rect: egui::Rect,
        style: SketchNodeStyle,
        rotation_offset_deg: i32,
    ) -> Vec<KiCadSymbolPinAnchor> {
        let mut anchors = Vec::new();
        for primitive in &self.primitives {
            let KiCadSymbolPrimitive::PinLine {
                pin: Some(pin),
                start,
                end,
            } = primitive
            else {
                continue;
            };
            let pos = self.project(*start, rect, style, rotation_offset_deg);
            let inner = self.project(*end, rect, style, rotation_offset_deg);
            let outward = pos - inner;
            let outward = if outward.length_sq() > f32::EPSILON {
                outward.normalized()
            } else {
                outward_from_center(pos, rect.center())
            };
            anchors.push(KiCadSymbolPinAnchor {
                pin: pin.clone(),
                pos,
                label_pos: pos + outward * 10.0,
                label_align: align_for_outward(outward),
            });
        }
        anchors.sort_by(|left, right| left.pin.cmp(&right.pin));
        anchors.dedup_by(|left, right| left.pin == right.pin);
        anchors
    }

    fn project(
        &self,
        point: KiCadPoint,
        rect: egui::Rect,
        style: SketchNodeStyle,
        rotation_offset_deg: i32,
    ) -> egui::Pos2 {
        let width = (self.max.x - self.min.x).abs().max(f32::EPSILON);
        let height = (self.max.y - self.min.y).abs().max(f32::EPSILON);
        let scale = (rect.width() / width).min(rect.height() / height);
        let mut delta = egui::vec2(
            (point.x - (self.min.x + self.max.x) * 0.5) * scale,
            -(point.y - (self.min.y + self.max.y) * 0.5) * scale,
        );
        if style.mirrored {
            delta.x = -delta.x;
        }
        let radians = (style.rotation_deg + rotation_offset_deg).rem_euclid(360) as f32
            * std::f32::consts::TAU
            / 360.0;
        let rotated = egui::vec2(
            delta.x * radians.cos() - delta.y * radians.sin(),
            delta.x * radians.sin() + delta.y * radians.cos(),
        );
        rect.center() + rotated
    }
}

fn outward_from_center(pos: egui::Pos2, center: egui::Pos2) -> egui::Vec2 {
    let outward = pos - center;
    if outward.length_sq() > f32::EPSILON {
        outward.normalized()
    } else {
        egui::vec2(1.0, 0.0)
    }
}

fn align_for_outward(outward: egui::Vec2) -> egui::Align2 {
    if outward.x.abs() >= outward.y.abs() {
        if outward.x < 0.0 {
            egui::Align2::RIGHT_CENTER
        } else {
            egui::Align2::LEFT_CENTER
        }
    } else if outward.y < 0.0 {
        egui::Align2::CENTER_BOTTOM
    } else {
        egui::Align2::CENTER_TOP
    }
}

impl KiCadSymbolPrimitive {
    fn collect_points(&self, points: &mut Vec<KiCadPoint>) {
        match self {
            Self::Polyline(polyline) => points.extend(polyline.iter().copied()),
            Self::Rectangle { start, end } => {
                points.push(*start);
                points.push(*end);
            }
            Self::Circle { center, radius } => {
                points.push(KiCadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                points.push(KiCadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc { start, mid, end } => {
                points.push(*start);
                points.push(*mid);
                points.push(*end);
            }
            Self::Text { at, .. } => points.push(*at),
            Self::PinLine { start, end, .. } => {
                points.push(*start);
                points.push(*end);
            }
        }
    }

    fn draw(&self, context: &KiCadDrawContext<'_>) {
        match self {
            Self::Polyline(points) => draw_polyline(
                context.painter,
                context.drawing,
                context.rect,
                context.style,
                context.rotation_offset_deg,
                context.stroke,
                points,
            ),
            Self::Rectangle { start, end } => {
                let corners = [
                    *start,
                    KiCadPoint {
                        x: end.x,
                        y: start.y,
                    },
                    *end,
                    KiCadPoint {
                        x: start.x,
                        y: end.y,
                    },
                    *start,
                ];
                draw_polyline(
                    context.painter,
                    context.drawing,
                    context.rect,
                    context.style,
                    context.rotation_offset_deg,
                    context.stroke,
                    &corners,
                );
            }
            Self::Circle { center, radius } => {
                let screen_center = context.drawing.project(
                    *center,
                    context.rect,
                    context.style,
                    context.rotation_offset_deg,
                );
                let edge = context.drawing.project(
                    KiCadPoint {
                        x: center.x + radius,
                        y: center.y,
                    },
                    context.rect,
                    context.style,
                    context.rotation_offset_deg,
                );
                context.painter.circle_stroke(
                    screen_center,
                    screen_center.distance(edge),
                    context.stroke,
                );
            }
            Self::Arc { start, mid, end } => {
                let points = approximate_arc(*start, *mid, *end);
                draw_polyline(
                    context.painter,
                    context.drawing,
                    context.rect,
                    context.style,
                    context.rotation_offset_deg,
                    context.stroke,
                    &points,
                );
            }
            Self::Text { text, at } => {
                let pos = context.drawing.project(
                    *at,
                    context.rect,
                    context.style,
                    context.rotation_offset_deg,
                );
                context.painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::monospace(11.0),
                    context.color,
                );
            }
            Self::PinLine { start, end, .. } => draw_polyline(
                context.painter,
                context.drawing,
                context.rect,
                context.style,
                context.rotation_offset_deg,
                context.stroke,
                &[*start, *end],
            ),
        }
    }
}

fn draw_polyline(
    painter: &egui::Painter,
    drawing: &KiCadSymbolDrawing,
    rect: egui::Rect,
    style: SketchNodeStyle,
    rotation_offset_deg: i32,
    stroke: egui::Stroke,
    points: &[KiCadPoint],
) {
    if points.len() < 2 {
        return;
    }
    let screen_points: Vec<_> = points
        .iter()
        .map(|point| drawing.project(*point, rect, style, rotation_offset_deg))
        .collect();
    painter.add(egui::Shape::line(screen_points, stroke));
}

fn approximate_arc(start: KiCadPoint, mid: KiCadPoint, end: KiCadPoint) -> Vec<KiCadPoint> {
    let Some((center, radius)) = circle_from_three_points(start, mid, end) else {
        return vec![start, mid, end];
    };
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let mid_angle = (mid.y - center.y).atan2(mid.x - center.x);
    let end_angle = (end.y - center.y).atan2(end.x - center.x);
    let mut ccw_end = end_angle;
    while ccw_end < start_angle {
        ccw_end += std::f32::consts::TAU;
    }
    let mut ccw_mid = mid_angle;
    while ccw_mid < start_angle {
        ccw_mid += std::f32::consts::TAU;
    }
    let (from, to) = if ccw_mid <= ccw_end {
        (start_angle, ccw_end)
    } else {
        let mut cw_end = end_angle;
        while cw_end > start_angle {
            cw_end -= std::f32::consts::TAU;
        }
        (start_angle, cw_end)
    };
    (0..=16)
        .map(|index| {
            let t = index as f32 / 16.0;
            let angle = from + (to - from) * t;
            KiCadPoint {
                x: center.x + angle.cos() * radius,
                y: center.y + angle.sin() * radius,
            }
        })
        .collect()
}

fn circle_from_three_points(
    a: KiCadPoint,
    b: KiCadPoint,
    c: KiCadPoint,
) -> Option<(KiCadPoint, f32)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() <= f32::EPSILON {
        return None;
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let center = KiCadPoint {
        x: (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        y: (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    };
    Some((center, ((a.x - center.x).hypot(a.y - center.y))))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LIB: &str = r#"
(kicad_symbol_lib
  (version 20240101)
  (generator circuitci-test)
  (symbol "R"
    (symbol "R_0_1"
      (rectangle (start -1 -2) (end 1 2) (stroke (width 0.254) (type default)) (fill (type none)))
      (polyline (pts (xy -2 0) (xy -1 0)) (stroke (width 0.254) (type default)) (fill (type none)))
      (polyline (pts (xy 1 0) (xy 2 0)) (stroke (width 0.254) (type default)) (fill (type none)))
    )
    (symbol "R_1_1"
      (pin passive line (at 0 3 270) (length 1) (name "") (number "1"))
      (pin passive line (at 0 -3 90) (length 1) (name "") (number "2"))
    )
  )
)
"#;

    #[test]
    fn parses_kicad_symbol_drawing_primitives_and_pins() {
        let drawing = parse_kicad_symbol_drawing(TEST_LIB, "R").unwrap();
        assert!(drawing.primitives.len() >= 5);
        assert!(drawing.max.y >= 3.0);
        assert!(drawing.min.y <= -3.0);
    }

    #[test]
    fn projects_kicad_pin_lines_as_named_screen_anchors() {
        let drawing = parse_kicad_symbol_drawing(TEST_LIB, "R").unwrap();
        let anchors = drawing.pin_anchors(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(120.0, 120.0)),
            SketchNodeStyle::default(),
            0,
        );
        let by_pin = anchors
            .iter()
            .map(|anchor| (anchor.pin.as_str(), anchor))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(anchors.len(), 2);
        assert!(by_pin["1"].pos.y < by_pin["2"].pos.y);
        assert_eq!(by_pin["1"].label_align, egui::Align2::CENTER_BOTTOM);
        assert_eq!(by_pin["2"].label_align, egui::Align2::CENTER_TOP);
    }

    #[test]
    fn parses_kicad_symbol_catalog_entries_and_pin_numbers() {
        let entries = parse_kicad_symbol_catalog(TEST_LIB, "Device", "test.kicad_sym").unwrap();
        let resistor = entries.iter().find(|entry| entry.id == "Device:R").unwrap();

        assert_eq!(resistor.library, "Device");
        assert_eq!(resistor.name, "R");
        assert_eq!(resistor.source, "test.kicad_sym");
        assert!(resistor.pins.iter().any(|pin| pin.id == "1"));
        assert!(resistor.pins.iter().any(|pin| pin.id == "2"));
    }

    #[test]
    fn installed_kicad_library_loads_common_symbols_when_available() {
        if installed_kicad_symbol_library_paths().is_empty() {
            return;
        }
        let cache = load_default_symbol_cache();
        assert!(cache.contains_key("Device:R"));
        assert!(cache.contains_key("Device:C"));
        assert!(cache.contains_key("Device:L"));
        assert!(cache.contains_key("Device:D"));
        assert!(cache.contains_key("Device:Voltmeter_DC"));
        assert!(cache.contains_key("Device:Ammeter_DC"));
        assert!(cache.contains_key("Device:Oscilloscope"));
    }
}
