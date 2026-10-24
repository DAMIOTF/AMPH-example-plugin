# AMPH Example Plugin — `example.sizes`

[English version](README.en.md)

Samodzielne, minimalne repozytorium przykładowego pluginu dla
[AMPH](https://github.com/DAMIOTF/Arch-Meta-Package-Harmonizer). Pokazuje
kompletny kontrakt pluginu **functionality** dla AMPH 0.3.0-beta:

- manifest `plugin.json` w schemacie **1**;
- ABI **v1** (`amph_abi_version`, `amph_alloc`, `amph_analyze`);
- analizę migawki pakietów przekazywanej przez AMPH;
- binarny moduł WebAssembly dla `wasm32-unknown-unknown`;
- brak importów, WASI, `wasm-bindgen` i jakichkolwiek uprawnień hosta;
- publikację przez GitHub Release z assetami `plugin.json` i `plugin.wasm`
  oraz digestami `sha256:<64 hex>` generowanymi przez GitHub.

Repozytorium jest niezależne od checkoutu AMPH i nie modyfikuje go.

AMPH domyślnie podłącza prawdziwy menedżer pluginów (`StorePluginHandler`).
Po instalacji plugin pojawia się w menedżerze i można nim zarządzać z TUI.

## Jedno repozytorium = jeden plugin

AMPH pobiera `plugin.json` z **najnowszego stabilnego** GitHub Release danego
repozytorium. Release może zawierać tylko jeden manifest `plugin.json`, więc
jedno repozytorium publikuje **jeden plugin naraz**.

Dlatego:

- root tego repozytorium publikuje plugin functionality `example.sizes`;
- `examples/theme/plugin.json` to wyłącznie materiał referencyjny i **nie jest**
  dołączany do release'u;
- motyw trzeba opublikować w osobnym repozytorium (albo w osobnym repozytorium
  zastąpić rootowy manifest manifestem motywu).

Nie dodawaj dwóch manifestów ani dwóch pluginów do jednego release'u. Backend
wybiera dokładnie jeden asset o nazwie `plugin.json`.

## Spis treści

- [Szybki start](#szybki-start)
- [Struktura repozytorium](#struktura-repozytorium)
- [Manifest `plugin.json`](#manifest-pluginjson)
- [Bezpieczeństwo i sandbox](#bezpieczeństwo-i-sandbox)
- [ABI v1 i dane wejściowe](#abi-v1-i-dane-wejściowe)
- [Przykładowy analyzer](#przykładowy-analyzer)
- [Budowanie poprawnego modułu Wasm](#budowanie-poprawnego-modułu-wasm)
- [Wydanie na GitHubie](#wydanie-na-githubie)
- [Menedżer pluginów w AMPH](#menedżer-pluginów-w-amph)
- [Limity i błędy](#limity-i-błędy)
- [Kompatybilność](#kompatybilność)
- [Rozwój, testy i CI](#rozwój-testy-i-ci)
- [Licencja](#licencja)

## Szybki start

Wymagania: Rust z targetem `wasm32-unknown-unknown`, `cargo`, opcjonalnie
`wasm-tools` albo WABT (`wasm-validate` + `wasm2wat`). Do publikacji: GitHub
CLI (`gh`) albo interfejs GitHub.

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib

# Buduje dist/plugin.json i dist/plugin.wasm oraz sprawdza moduł.
./scripts/build-release-assets.sh

# Po utworzeniu Release:
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin
```

Instalacja w AMPH:

```text
Shift+P -> zakładka "Add" -> wklej https://github.com/DAMIOTF/AMPH-example-plugin
        -> Enter (review) -> Enter (install)
```

Potem w sekcji **Installed**:

- `a` — analiza pakietów (functionality; wybór i potwierdzenie),
- `space` — włącz/wyłącz plugin,
- `r` — usuń plugin (z potwierdzeniem),
- `T` — zastosuj motyw do bieżącej sesji,
- `c` — sprawdź aktualizacje,
- `U` — zaktualizuj wszystkie dostępne pluginy.

Adresy w tym README zakładają repozytorium
`DAMIOTF/AMPH-example-plugin`. Jeśli repo powstanie pod inną nazwą, podmień
`DAMIOTF/AMPH-example-plugin` w `Cargo.toml`, README, skryptach i workflow.

## Struktura repozytorium

```text
AMPH-example-plugin/
├── Cargo.toml                       # cdylib + rlib, profil release dla Wasm
├── Cargo.lock                       # brak zależności runtime
├── plugin.json                      # JEDYNY publikowany manifest
├── src/
│   └── lib.rs                       # ABI v1, parser snapshotu, testy
├── examples/
│   └── theme/
│       ├── plugin.json              # przykład motywu, NIE jest publikowany
│       └── README.md                # jak opublikować motyw osobno
├── scripts/
│   ├── build-release-assets.sh      # builduje i stage'uje dist/
│   ├── check-wasm.sh                # importy/start/eksporty/rozmiar
│   └── verify-release.sh            # digesty i pobranie assetów z GitHuba
├── .github/
│   └── workflows/
│       ├── ci.yml                   # format, testy, build, walidacja
│       └── release.yml              # stabilny tag -> GitHub Release
├── README.md                        # ten dokument (polski)
├── README.en.md                     # wersja angielska
└── LICENSE                          # MIT
```

`plugin.json` w rootcie jest manifestem functionality. Plik
`examples/theme/plugin.json` ma inny `id` (`example.theme`) i służy tylko jako
wzór; workflow release'owy go nie pakuje.

## Manifest `plugin.json`

Manifest jest **ścisłym** JSON-em. Backend odrzuca nieznane pola, złe typy,
niebezpieczne identyfikatory i pola niezgodne z rodzajem pluginu. Manifest jest
assetem release'u o dokładnie nazwie `plugin.json` i ma maksymalnie **64 KiB**.

### Pola wspólne

| Pole | Wymagane | Zasady |
|---|---:|---|
| `schema` | tak | Dokładnie `1`. Inna wartość jest odrzucana. |
| `id` | tak | 1–64 znaków; małe litery ASCII, cyfry, `-`, `_`, `.`; pierwszy i ostatni znak alfanumeryczny; bez `..`. Służy jako nazwa katalogu i klucz. |
| `name` | tak | 1–80 znaków, bez wiodących/końcowych spacji i znaków kontrolnych. |
| `version` | tak | 1–64 znaków; litery ASCII, cyfry, `.`, `-`, `+`, `_`. To wersja pluginu, nie ABI. |
| `kind` | tak | `"theme"` albo `"functionality"`. |
| `description` | nie | 0–512 znaków, bez znaków kontrolnych. |
| `author` | nie | 0–80 znaków, bez znaków kontrolnych. |

Pola `api`, `entry` i `theme` są zależne od `kind`:

- `theme`: wymaga `theme`; **zabrania** `api` i `entry`;
- `functionality`: wymaga `api: 1` i `entry` kończącego się `.wasm`; **zabrania**
  `theme`.

`entry` to sama nazwa pliku assetu, bez ścieżek (`/`, `\`, NUL), 1–128 znaków,
dozwolone litery ASCII, cyfry, `.`, `-`, `_`, `+`. Nazwy `plugin.json`,
`install.json` i `disabled` są zarezerwowane.

### Manifest functionality (publikowany tutaj)

```json
{
  "schema": 1,
  "id": "example.sizes",
  "name": "Example Snapshot Analyzer",
  "version": "1.0.0",
  "kind": "functionality",
  "description": "Summarizes the AMPH package snapshot: totals, largest package, explicit and foreign counts, and dependency links.",
  "author": "AMPH example contributors",
  "api": 1,
  "entry": "plugin.wasm"
}
```

Release musi zawierać asset `plugin.json` i asset o nazwie z `entry` — tutaj
`plugin.wasm`. Oba muszą mieć digest GitHuba.

### Manifest motywu (przykład referencyjny)

```json
{
  "schema": 1,
  "id": "example.theme",
  "name": "Example Gruvbox Theme",
  "version": "1.0.0",
  "kind": "theme",
  "theme": {
    "palette": "gruvbox",
    "accent": "#83A598",
    "good": "#B8BB26",
    "warn": "#FABD2F",
    "bad": "#FB4934",
    "muted": "#928374",
    "info": "#458588"
  }
}
```

Zasady motywu:

- `theme.palette` musi być jednym z wbudowanych: `catppuccin`, `tokyo-night`,
  `gruvbox`, `nord` (backend akceptuje też aliasy `mocha`, `tokyo`,
  `tokyonight`);
- `accent`, `good`, `warn`, `bad`, `muted`, `info` przyjmują **dokładnie**:
  - nazwę bazowego koloru ANSI: `black`, `red`, `green`, `yellow`, `blue`,
    `magenta`, `cyan`, `white`, `gray`/`grey`, `dark gray`/`dark grey`,
    `bright black`, `bright red`, `bright green`, `bright yellow`,
    `bright blue`, `bright magenta`, `bright cyan`, `bright white`, albo
  - dokładny kolor `#RRGGBB`: znak `#` i sześć cyfr szesnastkowych (wielkość
    liter nie ma znaczenia); skróty `#RGB`, `rgb(...)` i dowolne nazwy CSS są
    odrzucane;
- wartości domyślne to `cyan`, `green`, `yellow`, `red`, `bright black`,
  `blue`;
- obiekt `theme` odrzuca nieznane pola;
- motyw nie zawiera kodu, więc nie ma `api` ani `entry`.

Backend waliduje kolory już przy instalacji manifestu i ponownie przy
zastosowaniu. Motyw definiuje wyłącznie semantyczne kolory interfejsu. **Nie**
definiuje własnych ekranów, widżetów, poleceń, skrótów, uprawnień ani
zachowania TUI.

### Czego manifest nie może zawierać

Nie istnieją pola uprawnień. Manifest nie może zadeklarować dostępu do
systemu plików, sieci, procesów, zmiennych środowiskowych, zegara, schowka ani
callbacków hosta. Nie ma skryptów instalacyjnych, URL-i zależności, hooków
`preinstall`/`postinstall`, poleceń powłoki ani definicji ekranów. Próba dodania
np. `"permissions": ["network"]` kończy się błędem `unknown field`.

## Bezpieczeństwo i sandbox

Functionality plugin jest **bezstanowym hookiem analyze**. AMPH uruchamia go w
interpreterze `wasmi`, nie w JIT i nie natywnie. Host wywołuje analyze tylko
wtedy, gdy użytkownik jawnie wybierze włączony plugin functionality i potwierdzi
akcję `a`. Sama instalacja, odświeżenie listy, check aktualizacji ani start AMPH
nie wysyłają migawki pakietów do pluginu.

Migawka przekazywana przy potwierdzeniu zawiera wyłącznie dane pakietów:
`name`, `version`, `size`, `explicit`, `foreign`, `depends`, `provides` oraz
`scanned_at`. Nie zawiera ścieżek, uchwytów, zmiennych środowiskowych,
konfiguracji, historii pacmana ani żadnych możliwości hosta. Użytkownik widzi
ekran potwierdzenia z liczbą pakietów przed wysłaniem.

Najważniejsze reguły sandboxu:

- **zero importów** — linker hosta jest pusty; moduł z jakimkolwiek importem,
  w tym `wasi_snapshot_preview1`, jest odrzucany;
- **brak funkcji startowej** — `(start ...)` jest zabronione;
- **tylko pamięć 32-bitowa** — brak `memory64`;
- **brak WASI** — nie ma args, env, clock, random, file descriptors;
- **brak referencji, SIMD, tail calls, extended const, custom page sizes,
  multi-memory i wide arithmetic**;
- dozwolony jest praktyczny podzbiór core Wasm: mutable globals, sign
  extension, saturating float-to-int, multi-value, bulk memory i liczby
  zmiennoprzecinkowe;
- **brak trwałego stanu** — każdy `amph_analyze` dostaje świeży `Store` i
  instancję; pamięć i globals nie przeżywają wywołania;
- **brak dostępu do hosta** — plugin nie może wywołać żadnej funkcji hosta;
- **ograniczone zasoby** — rozmiar modułu, pamięć, wejście, wyjście, fuel,
  rekurencja i liczba instancji są ograniczone;
- **integralność** — instalacja wymaga HTTPS, dokładnego URL-a GitHub,
  najnowszego stabilnego release'u i zgodnego digestu SHA-256 dla każdego
  wybranego assetu;
- **atomowość** — instalacja/aktualizacja odbywa się przez katalog staging i
  `rename`; usunięcie najpierw przenosi katalog poza widoczną przestrzeń;
- **brak symlinków** — odrzucany jest katalog pluginu będący symlinkiem oraz
  symlinki plików w katalogu pluginu: `plugin.json`, `install.json`, markera
  `disabled` i wpisu `.wasm` functionality.

To nie jest piaskownica procesu systemowego z uprawnieniami POSIX. To znacznie
węższy model: brak interfejsu z hostem i brak I/O. Mimo tego plugin jest
niezaufanym kodem — przed instalacją sprawdź źródło/repozytorium. Ekran
potwierdzenia analizy jest ostatnią linią obrony, nie analizą bezpieczeństwa.

### Limity runtime

| Zasób | Limit |
|---|---:|
| Rozmiar manifestu | 64 KiB |
| Rozmiar modułu Wasm | 2 MiB |
| Pamięć liniowa | 16 MiB |
| Wejście JSON | 1 MiB |
| Wyjście JSON | 1 MiB |
| Fuel na jedno `amph_analyze` | 20 000 000 |
| Maksymalna rekurencja | 256 |
| Pamięci / tabele / instancje | 1 / 1 / 2 |
| Elementy tabeli | 1024 |
| Timeout sieci (API i asset) | 20 s |

Limity są egzekwowane na kilku warstwach: przed parsowaniem, przez limity
`wasmi`, przez deklarowaną pamięć, resource limiter, fuel oraz limity JSON.

## ABI v1 i dane wejściowe

### Eksporty

Moduł functionality musi eksportować co najmniej tę powierzchnię (szkic
sygnatur, nie pełny moduł):

```wat
(module
  (memory (export "memory") 1)                         ;; 32-bit linear memory
  (func (export "amph_abi_version") (result i32))      ;; musi zwrócić 1
  (func (export "amph_alloc") (param i32) (result i32))
  (func (export "amph_analyze") (param i32 i32) (result i64)))
```

Wymagane sygnatury:

| Eksport | Sygnatura | Znaczenie |
|---|---|---|
| `memory` | pamięć liniowa 32-bit | Bufor wejścia/wyjścia. |
| `amph_abi_version` | `() -> i32` | Musi zwrócić `1`; sprawdzane przy każdym analyze. |
| `amph_alloc` | `(i32) -> i32` | Rezerwuje `len` bajtów i zwraca wskaźnik; wartość ujemna oznacza błąd. |
| `amph_analyze` | `(i32, i32) -> i64` | Otrzymuje `(ptr, len)` JSON-a i zwraca spakowany wynik. |

Moduł nie może mieć importów ani startu. Backend wymaga też eksportu
`memory`. Samo poprawne skompilowanie nie wystarczy — sygnatury są sprawdzane.

### Wejście analyze

Dokładny envelope wysyłany przez AMPH po potwierdzeniu `a`:

```json
{
  "api": 1,
  "op": "analyze",
  "snapshot": {
    "scanned_at": "2026-10-25 00:00",
    "packages": [
      {
        "name": "linux",
        "version": "6.12.1",
        "size": 1073741824,
        "explicit": true,
        "foreign": false,
        "depends": ["glibc", "bash"],
        "provides": []
      },
      {
        "name": "example-foreign",
        "version": "2.0",
        "size": 1048576,
        "explicit": false,
        "foreign": true,
        "depends": [],
        "provides": ["example"]
      }
    ]
  }
}
```

Pola pakietu:

| Pole | Typ | Znaczenie |
|---|---|---|
| `name` | string | Nazwa pakietu. |
| `version` | string | Zainstalowana wersja. |
| `size` | integer | Rozmiar w bajtach. |
| `explicit` | bool | Czy pakiet jest jawnie zainstalowany. |
| `foreign` | bool | Czy pakiet jest obcy (spoza repozytoriów). |
| `depends` | string[] | Nazwy zależności. |
| `provides` | string[] | Nazwy dostarczanych symboli. |

`snapshot.packages` to bieżący inventory AMPH; kolejność nie jest kontraktem.
Plugin powinien tolerować dodatkowe pola obiektów, żeby przyszły host mógł
dodać metadane. Ten przykład to robi, ale wymaga `api=1`, `op="analyze"` i
`snapshot.packages`.

### Przebieg wywołania

1. Host waliduje moduł i tworzy świeży `Store` oraz instancję.
2. Wywołuje `amph_abi_version()` i wymaga wartości `1`.
3. Wywołuje `amph_alloc(input.len())`, sprawdza wskaźnik i granice pamięci.
4. Kopiuje bajty JSON do pamięci pluginu.
5. Wywołuje `amph_analyze(ptr, len)`.
6. Rozpakowuje wynik:

```text
packed  = (output_ptr as u64) << 32 | output_len as u64
```

`output_ptr` to wysokie 32 bity, `output_len` to niskie 32 bity. Host sprawdza
limit wyjścia, granice pamięci i kopiuje bajty **przed** zniszczeniem
instancji.

7. Odpowiedź musi być **jedną poprawną wartością JSON**. Host parsuje ją przez
   `serde_json`; puste wyjście, kilka wartości lub bajty nie-JSON są błędem.
8. Brak stanu między wywołaniami: kolejny analyze dostaje nową instancję.

W ABI v1 nie ma kanału błędu. Jeśli plugin chce zgłosić problem, musi zwrócić
poprawny JSON. Backend nie interpretuje semantyki odpowiedzi; sprawdza tylko
poprawność i limity.

### Kontrakt odpowiedzi tego przykładu

Przykład zwraca obiekt z `summary` (string) i `findings` (tablica). TUI używa
`summary` jako krótkiego komunikatu, a `findings` jako szczegółów wyniku.

```json
{
  "summary": "Analyzed 2 packages; total 1 GiB; 1 explicit, 1 foreign; largest linux 6.12.1 (1 GiB).",
  "findings": [
    {
      "kind": "totals",
      "packages": 2,
      "bytes": 1074790400,
      "explicit": 1,
      "foreign": 1,
      "dependency_links": 2,
      "provide_links": 1
    },
    {
      "kind": "largest",
      "name": "linux",
      "version": "6.12.1",
      "bytes": 1073741824
    }
  ]
}
```

Dla pustej migawki:

```json
{"summary":"Analyzed 0 packages; the snapshot is empty.","findings":[{"kind":"totals","packages":0,"bytes":0,"explicit":0,"foreign":0,"dependency_links":0,"provide_links":0}]}
```

Dla złego requestu plugin zwraca poprawny JSON z `summary` i `detail`:

```json
{"summary":"Invalid AMPH analyze request","detail":"expected api=1, op=analyze and snapshot.packages"}
```

To nie jest formalny schemat narzucony przez AMPH dla wszystkich pluginów —
AMPH wymaga tylko jednej poprawnej wartości JSON. To kontrakt tego przykładu,
dopasowany do sposobu, w jaki TUI podsumowuje odpowiedź.

## Przykładowy analyzer

`src/lib.rs` jest kompletny i nie ma zależności zewnętrznych:

- `#![no_std]` w buildzie Wasm — brak `std`, WASI i alokatora systemowego;
- dwa statyczne bufory: 1 MiB wejścia i 256 KiB wyjścia;
- brak importów i brak funkcji startowej;
- ręczny parser JSON dla envelope `api/op/snapshot/packages`;
- toleruje zmianę kolejności pól i pomija nieznane pola;
- liczy: liczbę pakietów, sumę rozmiarów, explicit, foreign, liczbę wpisów
  `depends`/`provides` oraz największy pakiet;
- renderuje `summary` i `findings` bez alokacji;
- błąd requestu zwraca jako poprawny JSON.

Fragment istotny dla ABI:

```rust
#[no_mangle]
pub extern "C" fn amph_abi_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn amph_alloc(len: i32) -> i32 {
    // ... walidacja i wskaźnik do bufora ...
}

#[no_mangle]
pub extern "C" fn amph_analyze(pointer: i32, length: i32) -> i64 {
    // ... snapshot -> summary/findings, następnie (ptr << 32) | len ...
}
```

### Czego ten plugin nie może zrobić

- **nie** czyta plików, pacman DB, sieci, zmiennych środowiskowych ani zegara;
- **nie** otrzymuje ścieżek, uchwytów, uprawnień ani konfiguracji;
- **nie** zachowuje stanu między analizami;
- **nie** definiuje własnych ekranów, widżetów, poleceń ani skrótów TUI;
- **nie** może uruchomić analyze bez akcji użytkownika `a` i potwierdzenia;
- **nie** może zmienić motywu ani interfejsu — to robi wyłącznie `T` na
  podstawie deklaratywnego manifestu motywu.

## Budowanie poprawnego modułu Wasm

### Wymagania

- Rust stable; CI używa stable;
- target `wasm32-unknown-unknown`:

```sh
rustup target add wasm32-unknown-unknown
```

**Nie** używaj targetu WASI (`wasm32-wasip1` / `wasm32-wasi`) ani
`wasm-bindgen`. Moduł WASI albo wygenerowany z importami hosta zostanie
odrzucony przez AMPH.

### Build

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib
cargo build --release --target wasm32-unknown-unknown --locked
```

Artefakt:

```text
target/wasm32-unknown-unknown/release/amph_example_plugin.wasm
```

Skrypt release'owy robi to samo i stage'uje poprawne nazwy:

```sh
./scripts/build-release-assets.sh
# dist/plugin.json
# dist/plugin.wasm
```

Skrypt wymaga walidatora: `wasm-tools` albo WABT. Instalacja np.:

```sh
cargo install wasm-tools --locked
# albo na Arch Linux:
sudo pacman -S wabt
```

`check-wasm.sh` sprawdza: poprawność modułu, brak importów, brak `start`,
dokładnie jedną pamięć, eksport `memory` i trzy funkcje ABI oraz limit 2 MiB.
Pełną walidację sygnatur, pamięci i feature'ów wykonuje AMPH przy instalacji.

### Ręczna weryfikacja

```sh
wasm-tools validate dist/plugin.wasm
wasm-tools print dist/plugin.wasm | grep -E '\(import |\(start '
# Powyższe grep nie powinien nic wypisać.

wasm-tools print dist/plugin.wasm | grep -E '\(export "(memory|amph_abi_version|amph_alloc|amph_analyze)"'
# Powinny być cztery eksporty.

wasm-objdump -x dist/plugin.wasm   # jeśli używasz WABT
```

Moduł musi zaczynać się od bajtów `\0asm`. Plik WAT (tekstowy) nie jest
akceptowany jako `entry`.

### Dlaczego `cargo test --lib`

`cdylib` dla `wasm32-unknown-unknown` jest budowany jako `no_std` z
`panic = "abort"`, a testowy harness Rusta potrzebuje innego profilu. Dlatego
testy jednostkowe parsera/renderera uruchamiamy jako
`cargo test --locked --lib`. Build release'owy dla Wasm nadal używa `cdylib` i
`no_std`.

## Wydanie na GitHubie

### Zasady

- Publikuj tylko **stabilne** tagi, najlepiej `vMAJOR.MINOR.PATCH`, np.
  `v1.0.0`; workflow odrzuca `v1.0.0-rc.1`.
- `version` w `plugin.json` musi odpowiadać tagowi (`v` + wersja).
- Release nie może być draftem ani prerelease'em — AMPH używa
  `releases/latest`, które je pomija. „Latest” to oznaczenie GitHuba, nie
  automatycznie najwyższy semver; upewnij się, że właściwy release jest
  oznaczony jako latest.
- Assety pluginu functionality muszą mieć dokładnie nazwy `plugin.json` i
  `plugin.wasm`. Release motywu zawiera tylko `plugin.json` (bez Wasm), ale
  ten manifest również musi mieć digest.
- GitHub automatycznie wylicza digest `sha256:<64 hex>` dla każdego assetu.
  AMPH czyta ten digest z API. **Nie zastępuj go** plikiem
  `plugin.wasm.sha256`; taki plik nie jest używany.
- Każdy wybrany asset musi mieć digest. Brak digestu oznacza odmowę
  instalacji.
- Jeden release = jeden plugin. Nie dodawaj manifestu motywu z
  `examples/theme` do tego samego release'u.

### Workflow

`.github/workflows/release.yml` uruchamia się na tagu `v*`, weryfikuje stabilny
tag i zgodność wersji, buduje moduł, waliduje go, tworzy Release przez `gh` i
na końcu sprawdza digesty oraz pobrane bajty.

Wymagania po stronie repozytorium:

- GitHub Actions mogą tworzyć release'y; workflow ma
  `permissions: contents: write`;
- branch `main` zawiera zatwierdzony kod;
- tag wskazuje commit, z którego ma powstać release.

Po utworzeniu zdalnego repozytorium typowy przebieg wygląda tak:

```sh
git init -b main
git add .
git commit -m "Add AMPH example functionality plugin"
git remote add origin https://github.com/DAMIOTF/AMPH-example-plugin.git
git push -u origin main

git tag v1.0.0
git push origin v1.0.0
```

Workflow utworzy Release z:

```text
plugin.json
plugin.wasm
```

### Ręczne wydanie

```sh
./scripts/build-release-assets.sh
gh release create v1.0.0 \
  dist/plugin.json \
  dist/plugin.wasm \
  --title "Example Snapshot Analyzer v1.0.0" \
  --notes "Example AMPH functionality plugin (ABI v1)."
```

W interfejsie GitHub: **Releases → Draft a new release → wybierz tag → dołącz
`plugin.json` i `plugin.wasm` → Publish release**. Nie zaznaczaj „prerelease”,
jeśli chcesz, żeby AMPH go widział.

### Weryfikacja po publikacji

```sh
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin
```

Skrypt pobiera metadane najnowszego stabilnego release'u przez API, sprawdza
tag, brak draft/prerelease, nazwy assetów, obecność digestów, URL-e, rozmiary i
SHA-256 pobranych bajtów. Ponawia próbę przez chwilę, ponieważ GitHub może
wyliczać digest chwilę po uploadzie. Ustaw `GH_TOKEN`, jeśli grozi Ci limit API.

### Zmiana wydania

Nie podmieniaj po cichu assetów pod tym samym tagiem. Backend wykrywa przy
checku zmianę tagu release'u, digestu `plugin.json` oraz — dla functionality —
digestu wpisu Wasm, ale semantycznie nowa zawartość powinna dostać nową
wersję i nowy tag. Wydanie poprawki: podnieś `version` w `plugin.json` i
`Cargo.toml`, commit, tag `vX.Y.Z`, push tagu.

## Menedżer pluginów w AMPH

AMPH domyślnie podłącza `StorePluginHandler` i magazyn
`$XDG_DATA_HOME/amph/plugins` (fallback `~/.local/share/amph/plugins`).
Menedżer otwiera się przez `Shift+P` z każdej zakładki.

### Instalacja

1. Uruchom AMPH.
2. Naciśnij `Shift+P`.
3. Przejdź do sekcji **Add** (`2`).
4. Wklej **dokładnie** `https://github.com/DAMIOTF/AMPH-example-plugin`.
   Backend akceptuje tylko `https://github.com/{owner}/{repo}`, opcjonalnie z
   jednym końcowym `/`. Odrzuca HTTP, GitLab, ścieżki dodatkowe, query,
   fragment, dane logowania i inne hosty.
5. Używaj dokładnej wielkości liter `owner/repo` z GitHuba. Backend porównuje
   prefiks URL-a assetu z podanym URL-em i robi to rozróżniając wielkość
   liter; inna wielkość może dać błąd mimo działającego linku w przeglądarce.
6. Naciśnij `Enter`, aby przejrzeć operację, a następnie potwierdź `Enter`,
   aby zainstalować.
7. AMPH pobiera najnowszy stabilny release, weryfikuje digesty, waliduje
   manifest i moduł, a dopiero potem publikuje katalog pluginu atomowo.

Po instalacji plugin znajduje się w:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/amph/plugins/example.sizes/
├── plugin.json       # zweryfikowany manifest
├── plugin.wasm       # entry functionality
├── install.json      # provenance: repo, tag, czas, SHA-256
└── disabled          # tylko gdy plugin jest wyłączony
```

`install.json` zawiera m.in. `repository`, `release_tag`, `manifest_sha256` i
`wasm_sha256`. Jest zapisem pochodzenia; AMPH używa go do check/update.

### Skróty w sekcji Installed

| Klawisz | Działanie |
|---|---|
| `j`/`k`, strzałki | wybór pluginu |
| `space` | włącz/wyłącz zaznaczony plugin |
| `a` | uruchom analyzer zaznaczonego włączonego pluginu functionality |
| `T` | zastosuj motyw zaznaczonego włączonego pluginu theme do bieżącej sesji |
| `r` | usuń zaznaczony plugin (potwierdzenie `Enter`) |
| `c` | sprawdź aktualizacje (przejście do **Updates**) |
| `U` | zaktualizuj wszystkie pluginy z dostępną aktualizacją |
| `R` | odśwież listę |
| `Tab`/`Shift+Tab` | zmiana sekcji |
| `Esc` | zamknij menedżer |

W sekcji **Add** działa wpisywanie/wklejanie URL-a, `Enter` (review) i `Esc`.
W sekcji **Updates** działają `c` (check), `Enter` lub `U` (update all);
pojedyncza aktualizacja wybranego wiersza nie jest osobną akcją.

### Analiza pakietów (`a`)

Dla zaznaczonego, **włączonego** pluginu functionality naciśnij `a`:

1. AMPH pokazuje ekran `ANALYZE PACKAGES` z nazwą pluginu i liczbą pakietów.
2. Jeszcze nic nie zostało wysłane. Dopiero `Enter` potwierdza przekazanie
   bieżącej migawki inventory do pluginu.
3. Host uruchamia moduł w sandboxie i pokazuje `summary`/`findings` z JSON-a.
4. `Esc` anuluje; analyze nie uruchomi się dla wyłączonego pluginu, motywu ani
   pluginu bez entry.

Migawka zawiera wyłącznie pakiet `name`, `version`, `size`, `explicit`,
`foreign`, `depends`, `provides` oraz `scanned_at`. Nie zawiera ścieżek,
uchwytów, konfiguracji ani możliwości hosta. To jedyny moment, w którym
inventory jest przekazywane do functionality pluginu.

To działanie nie tworzy własnego ekranu pluginu. TUI wyświetla tylko
ograniczony komunikat i szczegóły odpowiedzi; plugin nie może rysować UI,
otwierać okien ani uruchamiać poleceń.

### Motywy (`T`)

Dla zaznaczonego, **włączonego** pluginu theme naciśnij `T`:

- AMPH waliduje ponownie kolory i paletę z manifestu;
- motyw jest aplikowany **tylko do bieżącej sesji**;
- nic nie jest zapisywane do konfiguracji; po restarcie AMPH wraca do
  wcześniejszego motywu;
- `NO_COLOR` wygrywa — paleta pozostaje monochromatyczna;
- wyłączony plugin, plugin functionality albo brak danych motywu kończy się
  komunikatem, bez zmiany wyglądu.

To jest deklaratywna zmiana sześciu semantycznych kolorów, nie plugin UI.
Plugin nie może definiować własnych ekranów, widżetów ani dowolnych
możliwości.

### Sprawdzanie aktualizacji

- `c` w **Installed** lub **Updates** uruchamia `check_all`.
- Check pobiera metadane najnowszego stabilnego release'u i porównuje z
  `install.json`: tag release'u, digest `plugin.json` oraz — dla functionality —
  digest wybranego wpisu Wasm (`entry`).
- „Update available” oznacza zmianę tagu release'u, digestu `plugin.json` **lub**
  (dla functionality) digestu wpisu Wasm. To nie jest porównanie semver. Motyw
  nie ma wpisu Wasm, więc porównywane są dla niego tag i digest `plugin.json`.
- Plugin bez `install.json` jest raportowany jako `NotTracked` i nie da się go
  sprawdzić.
- Błędy pojedynczych pluginów są raportowane osobno; check nie przerywa
  całego menedżera.

### Aktualizacja wszystkich (`U`)

- `U` w **Installed** lub **Updates** aktualizuje wszystkie pluginy z
  dostępnymi aktualizacjami.
- `Enter` w **Updates** także uruchamia update all (nie ma osobnej akcji
  aktualizacji jednego wiersza).
- `update_all` kontynuuje po błędzie jednego pluginu i raportuje wynik dla
  każdego osobno.
- Aktualizacja zachowuje stan wyłączenia: jeśli katalog miał marker
  `disabled`, nowa wersja również go dostanie.
- Instalacja jest atomowa; nieudany update nie powinien zostawić częściowo
  zapisanego pluginu.

### Włączanie i wyłączanie (`space`)

W **Installed** zaznacz plugin i naciśnij `space`. AMPH wywołuje
`set_enabled` i zapisuje/usuwa pusty marker `disabled` w katalogu pluginu.
Stan jest trwały między uruchomieniami AMPH i zachowywany przy aktualizacji.

- wyłączony functionality plugin nie może uruchomić `a`;
- wyłączony theme plugin nie może zostać zastosowany przez `T`;
- wyłączenie nie usuwa pluginu ani nie zmienia manifestu;
- nie edytuj `plugin.json`, `install.json` ani markera ręcznie — użyj `space`.

### Usuwanie (`r`)

W **Installed** zaznacz plugin, naciśnij `r`, potem potwierdź `Enter`. AMPH
najpierw sprawdza manifest, potem przenosi katalog poza widoczną przestrzeń i
usuwa go. Usunięcie jest trwałe.

### Gdy magazyn pluginów jest nieczytelny

Jeśli AMPH nie może odczytać magazynu pluginów, uruchamia awaryjny handler
tylko do odczytu i pokazuje ostrzeżenie. To sytuacja awaryjna (np. brak
uprawnień, zły `XDG_DATA_HOME`/`HOME`, uszkodzony katalog), nie domyślny stan
AMPH. Sprawdź komunikaty i uprawnienia do `.../amph/plugins`.

## Limity i błędy

### Najczęstsze komunikaty

| Komunikat / fragment | Znaczenie i co zrobić |
|---|---|
| `plugin source must be an https://github.com/{owner}/{repo} URL` | Użyj dokładnego publicznego URL-a GitHub, bez ścieżek/query. |
| `latest GitHub release ... has no 'plugin.json' asset` | Ostatni **stabilny** release nie ma manifestu. Sprawdź draft/prerelease i nazwę assetu. |
| `latest GitHub release ... has no 'plugin.wasm' asset` | Manifest wskazuje `entry`, którego nie ma w release. Nazwa musi być identyczna. |
| `GitHub release asset ... has no SHA-256 digest` | GitHub nie zwrócił digestu. Poczekaj/sprawdź API; plik `.sha256` nie jest używany. |
| `release asset digest must use the sha256:<hex> form` | Digest z API ma zły format; problem release'u/metadanych. |
| `SHA-256 mismatch` | Pobrane bajty różnią się od digestu. Ponów; jeśli powtarza się, podejrzewaj podmieniony asset/cache. |
| `Wasm imports are not allowed` | Moduł importuje funkcje (WASI, wasm-bindgen, host). Przebuduj bez importów. |
| `plugin does not export 'memory'` | Dodaj eksport pamięci liniowej 32-bit. |
| `does not export 'amph_alloc'` / `amph_analyze` / `amph_abi_version` | Brak eksportu lub zła nazwa; sprawdź `#[no_mangle]` i `cdylib`. |
| `invalid signature` | Zła sygnatura funkcji ABI; porównaj z tabelą. |
| `plugin reported ABI N ... supports ABI 1` | Zwróć `1` z `amph_abi_version`. |
| `plugin returned invalid JSON` | Wyjście nie jest jedną wartością JSON. Zwróć np. obiekt z `summary`. |
| `plugin output is too large` | Wyjście przekracza 1 MiB. Zmniejsz odpowiedź. |
| `plugin input is too large` | Wejście przekracza 1 MiB. |
| `Wasm module is too large` | Moduł przekracza 2 MiB; włącz `opt-level = "s"`, LTO, strip. |
| `plugin initial memory ... exceeds` | Deklarowana pamięć przekracza 16 MiB. |
| `out of fuel` | Pętla/obliczenia przekroczyły 20 mln fuel. Ogranicz pracę. |
| `plugin 'x' is a theme plugin and has no JSON analyzer` | `a` działa tylko dla functionality. |
| `plugin 'x' is disabled` | Włącz plugin `space` przed analyze. |
| `plugin ... is a symbolic link` | Odrzucane są symlinki katalogu pluginu oraz plików `plugin.json`, `install.json`, markera `disabled` i wpisu `.wasm`; użyj zwykłych plików. |
| `functionality plugin 'x' has no Wasm entry` | Manifest/entry jest niespójny; przeinstaluj poprawny release. |
| `No plugin selected` | Zaznacz wiersz `j`/`k`. |
| `This plugin does not analyze` | `a` działa tylko dla functionality; motyw używa `T`. |
| `This plugin is not a theme` | `T` działa tylko dla theme. |
| `Theme not applied` | Sprawdź paletę i kolory; dozwolone są wyłącznie nazwy ANSI z dokumentacji lub `#RRGGBB`. |
| `cannot run curl` | Zainstaluj `curl`; AMPH używa go do GitHub API i assetów. |
| `could not read installed plugins` / `READ ONLY` | Awaryjny fallback, gdy magazyn jest nieczytelny; sprawdź uprawnienia/XDG. |

### Typowe problemy

**„Release istnieje, ale AMPH go nie widzi.”**
Sprawdź, czy release jest opublikowany (nie draft), nie jest prerelease'em i
jest oznaczony jako **latest**. `releases/latest` nie zwraca draftów ani
prerelease'ów.

**„Dodałem checksumę, a AMPH nadal narzeka na digest.”**
AMPH czyta pole `digest` z GitHub API, a nie plik obok assetu. GitHub nadaje
digest po uploadzie; skrypt `verify-release.sh` czeka na jego pojawienie się.

**„Działa lokalnie, ale nie w AMPH.”**
Sprawdź target: to musi być `wasm32-unknown-unknown`, nie WASI. Sprawdź brak
importów i `start`. Uruchom `scripts/check-wasm.sh` oraz walidację w AMPH.

**„`a` nie jest dostępne.”**
Zaznaczony plugin musi być włączony i mieć `kind: functionality`. Dla motywu
jest `T`, nie `a`. Sprawdź też, czy plugin został poprawnie zainstalowany.

**„`T` nie zmienia kolorów.”**
Plugin musi być włączonym motywem, a kolory muszą być zgodne z listą nazw ANSI
lub `#RRGGBB`. `T` działa tylko w bieżącej sesji i nie zapisuje konfiguracji.
Jeśli ustawiono `NO_COLOR`, interfejs pozostaje monochromatyczny.

**„Analyzer odrzuca request.”**
Host wysyła envelope `api/op/snapshot/packages`; nie uruchamiaj pluginu z
własnym JSON-em poza AMPH. Sprawdź, czy używasz aktualnego builda i czy plugin
jest w wersji z tej gałęzi.

**„Plugin nie aktualizuje się.”**
Jeśli tag, digest `plugin.json` i — dla functionality — digest wpisu Wasm są
takie same, backend uznaje plugin za aktualny. Podnieś wersję i utwórz nowy
tag/release.

**„Chcę theme i functionality w jednym repo.”**
Nie da się ich opublikować jednocześnie z jednego release'u. Rozdziel je na dwa
repozytoria.

**„Menedżer pokazuje READ ONLY.”**
To nie jest normalny stan. AMPH nie mogło odczytać magazynu; sprawdź
uprawnienia do katalogu pluginów i `XDG_DATA_HOME`/`HOME`, potem uruchom
ponownie.

## Kompatybilność

| Element | Wersja / wymaganie |
|---|---|
| Manifest schema | `1` |
| ABI | `1` |
| Analyze request | `api: 1`, `op: "analyze"`, `snapshot.packages` |
| Backend źródłowy | AMPH 0.3.0-beta `src/plugin/` + `StorePluginHandler` |
| Target | `wasm32-unknown-unknown`, binarny `.wasm` |
| Pamięć | 32-bit, maks. 16 MiB |
| Importy | zero |
| Start | brak |
| WASI | brak |
| Host capabilities | brak |
| TUI | `Shift+P`; `space`, `a`, `T`, `r`, `c`, `U`, `R` |
| GitHub | publiczne `https://github.com/{owner}/{repo}` |
| Release | najnowszy stabilny, bez draft/prerelease |
| Digest | `sha256:<64 hex>` dla `plugin.json` i entry |
| System | Linux/Arch (AMPH); `curl` wymagany |
| Licencja przykładu | MIT |

Uwagi:

- `schema` i `api` są wersjonowane osobno od `version` pluginu. `version` może
  być np. `1.2.3`, a `api` nadal `1`.
- Backend nie negocjuje wersji w dół. Moduł ABI 2 nie zainstaluje się w buildzie
  wspierającym ABI 1.
- Manifest jest strict: nowe pola w przyszłym schemacie będą odrzucane przez
  starszy backend. Nie dodawaj „na zapas” pól, których backend nie zna.
- Feature set jest celowo węższy niż „wszystko, co przyjmie przeglądarka”. Nie
  zakładaj SIMD, referencji, wątków, atomics, tail calls ani memory64.
- Plugin nie ma dostępu do pacman DB, plików, sieci ani zmiennych środowiskowych.
  Host przekazuje dane wyłącznie w jawnie potwierdzonym analyze.
- Motywy są danymi deklaratywnymi (sześć kolorów + paleta), a nie pluginami UI.
- Aktualizacja jest oparta na tagu, digestcie `plugin.json` i — dla
  functionality — digestcie wpisu Wasm, nie na semver. Zmiana tagu, digestu
  `plugin.json` lub wpisu Wasm może oznaczyć update; sama zmiana `version` bez
  zmiany tych elementów nie.

## Rozwój, testy i CI

### Lokalnie

```sh
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --lib
./scripts/build-release-assets.sh
./scripts/verify-release.sh v1.0.0 DAMIOTF/AMPH-example-plugin  # po publikacji
```

Testy jednostkowe obejmują parser envelope, kolejność pól, nieznane pola,
pustą migawkę, wartości maksymalne i odrzucanie błędnych requestów. Testy nie
wymagają targetu Wasm; build release'owy wymaga `wasm32-unknown-unknown` i
walidatora.

### CI

`.github/workflows/ci.yml` przy push/PR:

1. instaluje stable Rust z targetem Wasm i WABT;
2. sprawdza `cargo fmt`;
3. uruchamia `cargo check` i testy parsera;
4. buduje `dist/plugin.json` + `dist/plugin.wasm`;
5. waliduje manifest i moduł;
6. publikuje artefakt CI.

`.github/workflows/release.yml` przy stabilnym tagu `v*` wykonuje te same
kontrolki, tworzy GitHub Release i weryfikuje digesty.

### Zasady zmian

- Zmiana ABI wymaga nowej wersji ABI i świadomej zmiany hosta; nie zmieniaj
  sygnatur pod tym samym `api: 1`.
- Zmiana schematu manifestu wymaga nowego `schema`; stare backendy odrzucą
  nieznane pola.
- Nie dodawaj zależności runtime do modułu bez sprawdzenia importów i rozmiaru.
- Nie commituj `dist/` ani `target/`.
- Aktualizuj testy i README razem z kontraktem.

## Licencja

Kod przykładu jest na licencji **MIT** — zobacz [LICENSE](LICENSE). AMPH jest
osobnym projektem na licencji PolyForm Noncommercial 1.0.0; to repozytorium nie
jest częścią AMPH i nie zmienia jego licencji.
