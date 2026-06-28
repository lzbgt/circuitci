# TPD2EUSB30 USB ESD Observation

This fixture opens directly in the GUI as `TPD2EUSB30 USB ESD`. It exercises the
source-backed `vendor.ti.tpd2eusb30` model and the reduced
`CIRCUITCI_TPD2EUSB30_USB_ESD` generated-SPICE face.

The observation drives USB D+ and D- nets at normal full-speed idle-style levels
and checks that both protected pins remain below the source-backed `5.5 V`
reverse standoff limit. The macro-model includes the datasheet typical
`0.7 pF` IO-to-ground capacitance for each line.

This is not an ESD pulse, USB eye-margin, leakage, or layout sign-off model. It
is a quick executable check that the protection part is bound to the intended
lines and that normal operating voltage stays inside the modeled standoff range.
