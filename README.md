# RosterFlow

**Busplanning en dienstroosters voor busbedrijven** — een desktopprogramma voor Windows en macOS
(Tauri 2). Je voert de gegevens van je eigen bedrijf in (haltes, lijnen met hun rijtijden en
vertrektijden, bussen, chauffeurs) en RosterFlow plant daarmee de omlopen van de bussen, de
diensten van de chauffeurs en het weekrooster.

De planning werkt in vijf stappen, elk een tabblad:

| Stap | Tabblad | Wat je doet |
|---|---|---|
| 1 | **Routes** | De lijnen: haltes per richting, minuten tussen de haltes, vertrektijden per dagsoort |
| 2 | **Timeplan** | Rijtijden per tijdvak, ritten één voor één aanpassen, tijden kopiëren naar andere dagen |
| 3 | **Vehicles** | Omlopen (blokken) van de bussen; *Optimize* zoekt de goedkoopste indeling |
| 4 | **Drivers** | Diensten van de chauffeurs volgens de arbeidstijdregels (pauzes, dienstduur, aflossing) |
| 5 | **Roster** | Wie rijdt welke dienst en welke bus op welke dag; *Fill the roster* vult automatisch in |

Daarnaast heeft het programma eigen schermen voor de stamgegevens: **Lijnen**, **Haltes**,
**Vloot**, **Personeel** (met ziek- en betermelden, vakanties, cursussen), **Verlofaanvragen** (met
advies op basis van het maximum aantal mensen tegelijk vrij), **Uitzendkrachten** en
**Instellingen** (bedrijf, stallingen, feestdagen en schoolvakanties, personeelsregels, taal,
licht/donker, back-ups). Alles is te importeren en exporteren als spreadsheet (CSV), het hele
bedrijf als één bestand. Afdrukken: busgrafiek en dienstengrafiek als PDF, dienstkaarten voor de
chauffeurs (80 mm bonprinter of A4).

Talen: Nederlands, Engels, Duits, Frans, Pools, Russisch en Oekraïens.

Zie **[docs/HANDLEIDING.md](docs/HANDLEIDING.md)** voor de handleiding voor planners.

## Installeren

Download de installer van de laatste release:

- **Windows**: `RosterFlow_<versie>_x64-setup.exe` (of de `.msi` voor beheerde installaties).
- **macOS**: `RosterFlow_<versie>_universal.dmg` (Apple silicon en Intel). Zolang de app niet
  genotariseerd is: de eerste keer rechtsklikken op de app en *Open* kiezen.

De gegevens staan per gebruiker in de map van het programma
(`%APPDATA%\app.rosterflow.desktop` op Windows, `~/Library/Application Support/app.rosterflow.desktop`
op macOS): per bedrijf één bestand, plus 30 dagen aan dagelijkse kopieën.

## Ontwikkelen

Opbouw:

```
crates/rosterflow-core   de planningsmotor: bedrijf, netwerk, dienstregeling, omlopen, diensten, rooster
crates/rosterflow-api    de commando's van de pagina (JSON in en uit), opslag, vertalingen (locales/*.yml)
crates/rosterflow-dev    een kleine webserver: de pagina in een browser, voor ontwikkelen en testen
app/                     de desktop-app (Tauri 2): venster, opslaan/openen-dialogen, installers
ui/                      de pagina (Preact + htm, geen build-stap): ui/js/rosterflow is de planning,
                         ui/js/pages de stamgegevens
```

Benodigd: Rust (stable) en, voor de desktop-app, de [Tauri-vereisten](https://tauri.app/start/prerequisites/)
van je platform.

```sh
cargo test -p rosterflow-core -p rosterflow-api   # de tests
cargo run -p rosterflow-dev                       # de pagina op http://127.0.0.1:1420
cargo run -p rosterflow                           # de desktop-app
npx @tauri-apps/cli@2 build                       # installers (in app/), of via GitHub Actions
```

`ROSTERFLOW_DATA=<map>` laat de app of de ontwikkelserver een andere gegevensmap gebruiken.

## Releases

Een tag `v<versie>` (gelijk aan de versie in `Cargo.toml` en `app/tauri.conf.json`) bouwt via
GitHub Actions (`.github/workflows/release.yml`) de installers voor Windows en macOS en zet ze in
een concept-release. Ondertekenen voor macOS gebeurt zodra de Apple-secrets zijn ingesteld (zie de
workflow).
