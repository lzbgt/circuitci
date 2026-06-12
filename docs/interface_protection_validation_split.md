# Interface Protection Validation Split

`src/validation/interface_protection.rs` owns signal-conditioning channel checks,
clamp-only protection checks, and top-level dispatch for USB protection rules.

USB-specific helpers are split so new layout rules do not push the parent module
toward the repository line-count guard:

- `src/validation/interface_protection/usb_connector_findings.rs` owns report
  construction for USB connector coverage, component-clearance, and USB
  protection placement findings.
- `src/validation/interface_protection/usb_connector_clearance.rs` owns
  `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` orchestration and its local
  connector-to-component footprint/placement clearance geometry.
- `src/validation/interface_protection/usb_connector.rs` owns
  `USB_CONNECTOR_PROTECTION_VALID`, `USB_PROTECTION_PLACEMENT_VALID`,
  `USB_CONNECTOR_ORIENTATION_VALID`, and
  `USB_CONNECTOR_EDGE_PROXIMITY_VALID`, and
  `USB_CONNECTOR_BODY_OVERHANG_VALID` orchestration plus shared USB connector
  placement/protection/layout helpers.
- `src/validation/interface_protection/usb_route.rs` owns
  `USB_ROUTE_GEOMETRY_VALID`, `USB_VBUS_ROUTE_VALID`, and
  `USB_RETURN_PATH_VALID` orchestration.
- `src/validation/interface_protection/usb_route/geometry.rs` owns USB route
  geometry math.
- `src/validation/interface_protection/usb_route/findings.rs` owns USB route
  finding construction.

When adding a USB layout rule, keep rule orchestration in a focused USB module,
pure geometry in a geometry helper, and report text/evidence keys in a findings
helper.
