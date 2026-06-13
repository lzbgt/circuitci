# USB Route Validation Split

`USB_ROUTE_GEOMETRY_VALID` is split across focused modules so layout checks can grow
without turning the validator into a single large file.

- `src/validation/interface_protection/usb_route.rs` owns
  `USB_ROUTE_GEOMETRY_VALID`: data-route scenario parameter parsing, USB
  connector/protection lookup, route orchestration, and differential-pair
  consistency checks.
- `src/validation/interface_protection/usb_route/vbus.rs` owns
  `USB_VBUS_ROUTE_VALID`: VBUS route policy resolution, route width/length/via
  checks, and placement- or pad-aware VBUS protection route-distance checks.
- `src/validation/interface_protection/usb_route/return_path.rs` owns
  `USB_RETURN_PATH_VALID`: same-layer ground-zone coverage, filled-copper
  coverage, same-island pad/via contact evidence, stitching-via proximity, and
  filled-zone edge-clearance checks.
- `src/validation/interface_protection/usb_route/geometry.rs` owns route math:
  route length, component-to-route projection, route graph distance, width
  deltas, and differential-pair gap evidence.
- `src/validation/interface_protection/usb_route/findings.rs` owns report
  construction for USB data-route and return-path failures.
- `src/validation/interface_protection/usb_route/vbus_findings.rs` owns report
  construction for `USB_VBUS_ROUTE_VALID` failures, including placement-based
  and pad-aware VBUS protection route evidence.

When adding a new USB layout check, keep data-route orchestration in
`usb_route.rs`, VBUS-route policy in `vbus.rs`, return-path policy in
`return_path.rs`, pure geometry in `geometry.rs`, and user-facing finding
text/evidence keys in the matching findings module.
