# nRF52840 Source Notes

Retrieved on 2026-07-05.

## Original Documents

The canonical Nordic documentation site exposes the current nRF52840 Product
Specification as HTML at:

- <https://docs.nordicsemi.com/bundle/ps_nrf52840/page/keyfeatures_html5.html>
- <https://docs.nordicsemi.com/r/bundle/ps_nrf52840/page/recommended_op_conditions.html?contentId=Vsnd3xqXJuSjfnZs2eJVbg>
- <https://docs.nordicsemi.com/r/bundle/ps_nrf52840/page/abs_max_ratings.html?contentId=xAtFYUDanPEETVC57D0NoQ>
- <https://docs.nordicsemi.com/r/bundle/ps_nrf52840/page/pin.html?contentId=hMpuE~dcsjrrIlIt8ErcrQ>

Direct command-line retrieval of the Nordic pages and old Infocenter PDF URLs
returned HTTP 403 from Nordic's site on 2026-07-05, so the repository retains
a distributor-hosted PDF copy of Nordic's product specification as an auditable
fallback artifact:

| Document | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- |
| nRF52840 Product Specification PDF mirror | <https://www.farnell.com/datasheets/2577974.pdf> | `docs/research/datasheets/nordic/nrf52840-product-spec-farnell.pdf` | `abf1271580b9d25920607abd05e4ad39d1d89dcb3bde5f5f04f4e34091a76fa0` |

## Modeled Facts

- Normal-voltage `VDD` recommended operating range is `1.7 V` to `3.6 V`.
- High-voltage `VDDH` recommended operating range is `2.5 V` to `5.5 V`.
- USB `VBUS` regulator input recommended operating range is `4.35 V` to
  `5.5 V`.
- Operating ambient temperature range is `-40 C` to `85 C`.
- `P0.18` can be configured as reset and is exposed as `nRESET` in this static
  board-boundary model.
- `SWDCLK` and `SWDIO` are retained as debug/programming pins.
- `USB_DP`, `USB_DM`, `VBUS`, and the `ANT` RF pin are retained as named board
  boundary pins, but only the supply voltage ranges are checked by this model.

## Non-Modeled Facts

The first model deliberately does not encode GPIO thresholds or drive strength,
because the current official Nordic HTML pages accessible through search did
not expose the GPIO electrical table in a command-line retrievable form. It
also does not sign off high-voltage-mode regulator sequencing, DCDC inductor
or decoupling networks, USB signal integrity, antenna matching, NFC behavior,
RF protocol behavior, UICR reset configuration programming, firmware
execution, thermal limits, or transient current waveforms.
