# USB Route Validation Split

`USB_ROUTE_GEOMETRY_VALID` is split across focused modules so layout checks can grow
without turning the validator into a single large file.

- `src/validation/interface_protection/usb_route.rs` owns scenario parameter
  parsing, USB connector/protection lookup, and rule orchestration.
- `src/validation/interface_protection/usb_route/geometry.rs` owns route math:
  route length, component-to-route projection, route graph distance, width
  deltas, and differential-pair gap evidence.
- `src/validation/interface_protection/usb_route/findings.rs` owns report
  construction for USB route failures.

When adding a new USB layout check, keep data extraction in `usb_route.rs`, pure
geometry in `geometry.rs`, and user-facing finding text/evidence keys in
`findings.rs`.
