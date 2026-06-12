use super::{ParsedComponent, ParsedKicadNetlist, ParsedNet, ParsedNode};
use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum TextTarget {
    Value,
    Field(String),
}

pub(super) fn parse_kicad_netlist(path: &Path) -> Result<ParsedKicadNetlist> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read KiCad netlist {}", path.display()))?;
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<String> = Vec::new();
    let mut components = BTreeMap::new();
    let mut nets = Vec::new();
    let mut current_component: Option<ParsedComponent> = None;
    let mut current_net: Option<ParsedNet> = None;
    let mut text_target: Option<TextTarget> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = local_name(event.name().as_ref());
                let parent = stack.last().map(String::as_str);
                match (tag.as_str(), parent) {
                    ("comp", Some("components")) => {
                        let refdes = required_attr(&reader, &event, "ref")?;
                        if components.contains_key(&refdes) {
                            bail!("Duplicate KiCad component reference {refdes}.");
                        }
                        current_component = Some(ParsedComponent {
                            refdes,
                            value: None,
                            lib: None,
                            part: None,
                            fields: BTreeMap::new(),
                            pin_electrical_types: BTreeMap::new(),
                            in_bom: None,
                            unit: None,
                            units: Vec::new(),
                            instances: Vec::new(),
                        });
                    }
                    ("net", Some("nets")) => {
                        current_net = Some(ParsedNet {
                            code: attr_value(&reader, &event, "code")?.unwrap_or_default(),
                            name: attr_value(&reader, &event, "name")?.unwrap_or_default(),
                            nodes: Vec::new(),
                        });
                    }
                    ("value", Some("comp")) if current_component.is_some() => {
                        text_target = Some(TextTarget::Value);
                    }
                    ("field", Some("fields")) if current_component.is_some() => {
                        text_target =
                            Some(TextTarget::Field(required_attr(&reader, &event, "name")?));
                    }
                    ("libsource", Some("comp")) if current_component.is_some() => {
                        apply_libsource(&reader, &event, current_component.as_mut().unwrap())?;
                    }
                    ("node", Some("net")) if current_net.is_some() => {
                        push_node(&reader, &event, current_net.as_mut().unwrap())?;
                    }
                    _ => {}
                }
                stack.push(tag);
            }
            Ok(Event::Empty(event)) => {
                let tag = local_name(event.name().as_ref());
                let parent = stack.last().map(String::as_str);
                match (tag.as_str(), parent) {
                    ("libsource", Some("comp")) if current_component.is_some() => {
                        apply_libsource(&reader, &event, current_component.as_mut().unwrap())?;
                    }
                    ("node", Some("net")) if current_net.is_some() => {
                        push_node(&reader, &event, current_net.as_mut().unwrap())?;
                    }
                    ("field", Some("fields")) if current_component.is_some() => {
                        let name = required_attr(&reader, &event, "name")?;
                        current_component
                            .as_mut()
                            .unwrap()
                            .fields
                            .insert(name, String::new());
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(event)) => {
                if let (Some(component), Some(target)) =
                    (current_component.as_mut(), text_target.as_ref())
                {
                    let value = event
                        .xml_content()
                        .context("Failed to decode KiCad XML text")?
                        .trim()
                        .to_string();
                    match target {
                        TextTarget::Value => component.value = non_empty(value),
                        TextTarget::Field(name) => {
                            component.fields.insert(name.clone(), value);
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = local_name(event.name().as_ref());
                match tag.as_str() {
                    "value" | "field" => text_target = None,
                    "comp" => {
                        let Some(component) = current_component.take() else {
                            bail!("KiCad XML closed a component without opening one.");
                        };
                        components.insert(component.refdes.clone(), component);
                    }
                    "net" => {
                        let Some(net) = current_net.take() else {
                            bail!("KiCad XML closed a net without opening one.");
                        };
                        nets.push(net);
                    }
                    _ => {}
                }
                let popped = stack.pop();
                if popped.as_deref() != Some(tag.as_str()) {
                    bail!("Malformed KiCad XML nesting near closing tag {tag}.");
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => bail!("Failed to parse KiCad XML netlist: {error}"),
            _ => {}
        }
    }

    if components.is_empty() {
        bail!("KiCad netlist {} contains no components.", path.display());
    }
    if current_component.is_some() || current_net.is_some() || !stack.is_empty() {
        bail!(
            "KiCad netlist {} ended with unclosed XML tags.",
            path.display()
        );
    }
    Ok(ParsedKicadNetlist { components, nets })
}

fn apply_libsource(
    reader: &Reader<&[u8]>,
    event: &BytesStart<'_>,
    component: &mut ParsedComponent,
) -> Result<()> {
    component.lib = attr_value(reader, event, "lib")?;
    component.part = attr_value(reader, event, "part")?;
    Ok(())
}

fn push_node(reader: &Reader<&[u8]>, event: &BytesStart<'_>, net: &mut ParsedNet) -> Result<()> {
    net.nodes.push(ParsedNode {
        refdes: required_attr(reader, event, "ref")?,
        pin: required_attr(reader, event, "pin")?,
        pintype: attr_value(reader, event, "pintype")?,
    });
    Ok(())
}

fn attr_value(
    reader: &Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &str,
) -> Result<Option<String>> {
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.context("Malformed KiCad XML attribute")?;
        if local_name(attribute.key.as_ref()) == name {
            let value = attribute
                .decode_and_unescape_value(reader.decoder())
                .context("Failed to decode KiCad XML attribute")?
                .to_string();
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn required_attr(reader: &Reader<&[u8]>, event: &BytesStart<'_>, name: &str) -> Result<String> {
    match attr_value(reader, event, name)? {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => bail!(
            "KiCad XML <{}> is missing required attribute {name}.",
            local_name(event.name().as_ref())
        ),
    }
}

fn local_name(name: &[u8]) -> String {
    let text = String::from_utf8_lossy(name);
    text.rsplit(':').next().unwrap_or(&text).to_string()
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}
