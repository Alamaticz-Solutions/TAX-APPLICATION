# Bundled Font Assets

These files support the PDS typography contract. Keep the binaries,
license texts, source versions, and hashes together.

| Asset | Upstream | Version | Bytes | SHA-256 |
| --- | --- | --- | ---: | --- |
| `InterVariable.woff2` | `https://rsms.me/inter/font-files/InterVariable.woff2?v=4.1` | Inter 4.1 distribution | 352240 | `693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3` |
| `GeistMonoVariable.woff2` | `https://github.com/vercel/geist-font/blob/v1.7.2/fonts/GeistMono/webfonts/GeistMono%5Bwght%5D.woff2` | Geist 1.7.2 | 71596 | `afaacc4c5fbba89d2ebf7a02dc4070208540874592a5504d57175782fe893101` |
| `Poppins-Bold-latin.woff2` | `https://fonts.gstatic.com/s/poppins/v24/pxiByp8kv8JHgFVrLCz7Z1xlFd2JQEk.woff2` | Poppins v24 (Google Fonts publisher latin subset, weight 700) | 7848 | `197a3cbd7290c242c5c765268cdd69a9a39867fdc80cd13071f243a81c56fb76` |

All fonts use the SIL Open Font License 1.1 with no Reserved Font Names
(verified against upstream license texts 2026-07-22). The corresponding
unmodified license texts are retained as `Inter-OFL.txt`, `Geist-OFL.txt`,
and `Poppins-OFL.txt`.

Poppins carries display/headline roles per the W0 two-tier brand-typography
decision (2026-07-22): Poppins Bold is the PDS Health brand web face for
display roles; Inter remains the UI body/data workhorse. The Poppins file is
Google's publisher-built latin subset served from fonts.gstatic.com — it is
not a locally modified artifact.

The combined WOFF2 payload is 431684 bytes (Inter 352240 + Geist Mono 71596 +
Poppins 7848). The earlier research estimate of a 40-60 KB Inter subset does
not describe the current official publisher file: Poppins demonstrates the
publisher-subset strategy (7.8 KB for the latin Bold cut), while Inter still
ships the full variable file. Do not claim a smaller Inter budget until an
accepted, license-compliant Inter subset is implemented and measured.
