# Handleiding RosterFlow

Deze handleiding loopt de planning van een busbedrijf door van begin tot eind: eerst de
stamgegevens, dan de vijf stappen van de planning, en tot slot het dagelijkse werk (ziekte,
verlof, uitzendkrachten, afdrukken).

## 1. Een bedrijf beginnen

Bij de eerste start zie je het startscherm.

- **Nieuw bedrijf**: naam, korte naam (2–4 letters, voor het logo), kleuren, de hoofdstalling,
  het land (voor de feestdagen) en de valuta.
- **Voorbeeldbedrijf**: een verzonnen bedrijf met drie lijnen, twaalf bussen en ruim dertig
  medewerkers. Handig om alles eerst uit te proberen; je kunt het later gewoon verwijderen.
- **Een bedrijfsbestand openen**: een bestand dat met *Instellingen → Gegevens → Bedrijf opslaan als
  bestand* is gemaakt (bijvoorbeeld op een andere computer).

Je kunt meerdere bedrijven naast elkaar hebben; via **Ander bedrijf** onderin de zijbalk ga je
terug naar het startscherm.

Het bedrijf wordt na elke wijziging automatisch opgeslagen. Van elke dag waarop iets veranderde,
blijft 30 dagen een kopie bewaard (*Instellingen → Gegevens → Terugzetten*).

## 2. Stamgegevens

### Haltes

*Haltes* bevat elke halte waar je lijnen stoppen: code, naam, coördinaten (optioneel) en of het
een **aflospunt** is (een plek waar een chauffeur de bus aan een collega mag overdragen; eindpunten
en de stalling zijn dat altijd).

Importeren kan met een spreadsheet (CSV, met `;` of `,`) met de kolommen `Code`, `Name`,
`Latitude`, `Longitude` (Nederlandse kolomnamen als `Naam`, `Haltecode` worden ook herkend; een
GTFS-`stops.txt` werkt ook). Een halte met een bestaande code wordt bijgewerkt.

### Lijnen

Per lijn voer je in:

1. **Nummer, naam, kleur en soort dienst**: *Reguliere dienst* (elke dag), *Scholierenvervoer*
   (ma–vr op schooldagen), *Weekend en vrije tijd* (za, zo, feestdagen) of *Op afroep*.
2. **Route**: de haltes van de heenrichting in volgorde, met de minuten vanaf de vorige halte.
   Typ de naam of code van een halte; bestaat die nog niet, dan maakt RosterFlow hem aan. Voor de
   terugweg kun je *Heenrichting omgekeerd* gebruiken en daarna aanpassen.
3. **Vertrektijden** per dagsoort (ma–vr, zaterdag, zondag/feestdag): eerste en laatste vertrek,
   de frequentie en de minimale keertijd aan het eindpunt. Met *Per dagdeel* geef je elk deel van
   de dag een eigen frequentie (spits, dal, avond). Heeft de terugweg andere tijden, vink dan
   *De terugweg heeft eigen vertrektijden* aan.
4. **Stalling en bussen**: de stalling van de lijn, de leegrit ernaartoe (minuten en km) en welke
   bustypes de lijn mogen rijden.

Bovenaan zie je direct of de lijn kan rijden en hoeveel bussen (omlopen) ze per dagsoort vraagt.
Na *Opslaan* worden de omlopen opnieuw gemaakt; omlopen die veranderen moeten in het rooster
opnieuw worden ingevuld (RosterFlow meldt hoeveel).

Dezelfde lijneditor vind je in de planning onder het tabblad **Routes**.

### Vloot

Elke bus met wagennummer, kenteken, merk en model, grootte (midi, solo, geleed, dubbeldekker),
aandrijving, zitplaatsen, stalling en de periode dat de bus in de vloot is. Een bus **in de
werkplaats** (tot en met een datum) wordt die dagen niet ingepland.

### Personeel

Per medewerker: personeelsnummer, functie (alleen *Buschauffeur* wordt op diensten gepland),
contact, rijbewijs (D of D1) met geldigheid (code 95), aanvullende bevoegdheden (geleed,
dubbeldekker, hoogspanning voor elektrische bussen), typeopleidingen (als het bedrijf die
bijhoudt), contracturen en maximaal aantal dagen per week, uurloon, vaste vrije weekdagen,
voorkeur voor vroege of late diensten, en of iemand **reserve** is.

Bovenaan het formulier meld je iemand **ziek** (tot een datum of open tot betermelding), **beter**,
**met vakantie tot** of **op cursus tot**. Het rooster houdt daar meteen rekening mee.

## 3. De planning

Open de planning via **Planning** in de zijbalk. Bovenin staan de stappen als tabbladen, de dag
die je bekijkt (een weekdag: wat je plant geldt voor die weekdag elke week), ongedaan maken en
opnieuw, en de knoppen voor voorkeuren, afdrukken, zoeken en de rondleiding (**?**). Wijzigingen in
*Vehicles* en *Drivers* zijn een concept tot je op **SAVE** drukt; *DISCARD* gooit ze weg.

### Stap 1 – Routes

De lijneditor (zie hierboven), zonder de planning te verlaten.

### Stap 2 – Timeplan

Per lijn, dagsoort en richting:

- **Tijdvakken** met elk hun eigen rijtijden tussen de haltes (bijvoorbeeld langer in de spits).
- De **dienstregeling** als tabel: ritten toevoegen, verschuiven of verwijderen, tijden per halte
  typen.
- **Tijden kopiëren** naar andere dagen of richtingen.

Sla de lijn op met *Save*; omlopen die daardoor veranderen worden opnieuw gepland.

### Stap 3 – Vehicles (omlopen)

Elke rij is een bus met zijn ritten door de dag (heen en terug, leegritten en uitrukken/inrukken in
grijs). Je kunt ritten slepen tussen omlopen. **OPTIMIZE** zoekt de indeling met de minste bussen
en kosten; het resultaat verschijnt als voorstel met *ACCEPT* of *KEEP MINE*. Bovenin staan de
kerncijfers: aantal diensten en omlopen, voertuig- en personeelsefficiëntie en de totale kosten.

### Stap 4 – Drivers (diensten)

De diensten van de chauffeurs: welke stukken van welke omlopen iemand rijdt, met pauzes, aflossing
en reistijd. De **voorkeuren** (knop met schuifjes) bevatten de regels: maximale dienstduur,
pauzes, rust tussen diensten, aflospunten en -tijden, kosten per uur, per km en per bus. Harde
regels die geschonden worden staan onder *Issues* onderaan.

### Stap 5 – Roster (rooster)

Per dag van de komende week: welke bus en welke chauffeur elke omloop en dienst rijdt.

- **FILL THE ROSTER** vult de dag automatisch in volgens contracten, vrije dagen, voorkeuren,
  bevoegdheden en rusttijden.
- **REPEAT MON-FRI** neemt het rooster van deze dag over voor de andere werkdagen.
- Klik een chauffeur of bus aan en daarna een dienst (of sleep), om iemand handmatig in te delen.
- Rechtsklik op een dienst voor *Verwijder chauffeur*, *Geef aan een uitzendkracht* en **Print
  duty card** (de dienstkaart voor de chauffeur).
- Open diensten en ritten zonder bus staan rechts, met wat er ontbreekt.

## 4. Het dagelijkse werk

- **Ziekte**: *Personeel → medewerker → Ziek melden*. Open *Roster* en vul de dag opnieuw (of kies
  per dienst een vervanger); de reserve-chauffeurs komen eerst.
- **Verlof**: *Verlofaanvragen → Nieuwe aanvraag*. Bij elke aanvraag staat een advies (past het
  binnen het maximum aantal mensen tegelijk vrij, zijn er nog dagen over). *Goedkeuren* houdt de
  chauffeur die dagen uit het rooster.
- **Uitzendkrachten**: *Uitzendkrachten → Boeken* voor een dag (vanaf morgen tot zes weken vooruit),
  hele dag of bepaalde uren. Het rooster geeft hun de open diensten van die dag.
- **Bus defect**: *Vloot → bus → Vandaag naar de werkplaats*; het rooster kiest een andere bus.

## 5. Afdrukken en exporteren

- **Busgrafiek / dienstengrafiek** als PDF: in de planning, knop met het document-icoon.
- **Dienstkaart**: in *Roster*, rechtsklik op een dienst → *Print duty card* (80 mm of A4).
- **Spreadsheets**: *Haltes*, *Vloot* en *Personeel* hebben elk *Importeren* en *Exporteren* (CSV,
  opent in Excel, Numbers en LibreOffice).
- **Het hele bedrijf**: *Instellingen → Gegevens → Bedrijf opslaan als bestand*; terugzetten via het
  startscherm (*Een bedrijfsbestand openen*).

## 6. Instellingen

- **Bedrijf**: naam, logo-kleuren, land en valuta, hoofdstalling en andere stallingen.
- **Personeelsregels**: dagen per week, uren van een volledige werkweek, maximaal aantal chauffeurs
  tegelijk met vakantie, en of chauffeurs per busmodel een typeopleiding nodig hebben.
- **Kalender**: schoolvakanties (scholierenlijnen rijden dan niet) en extra feestdagen (dan rijdt de
  zondagsdienstregeling). Invoeren als `2026-07-18` t/m `2026-08-30`, of elk jaar als `07-18` t/m
  `08-30`. De wettelijke feestdagen van Nederland, België, Duitsland en het Verenigd Koninkrijk
  kent RosterFlow zelf.
- **Programma**: taal en licht/donker.
- **Gegevens**: bedrijfsbestand opslaan, terugzetten naar een eerdere dag, de gegevensmap.
