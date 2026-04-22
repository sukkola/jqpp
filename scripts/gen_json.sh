#!/usr/bin/env bash
# gen_json.sh — generate JSON test fixtures for jqt development
#
# Usage:
#   ./gen_json.sh [TYPE] [OPTIONS]
#
# Types:
#   repos           git repositories (language, stars, commits, branches)
#   sensors         IoT sensor readings (temperature, humidity, pressure)
#   packages        software package dependency graph
#   dns             DNS zone records (A, CNAME, MX, TXT, NS)
#   movies          film catalogue with genres, ratings, cast
#   processes       OS process list (pid, cpu, mem, state)
#   matrix          2-D numeric matrix
#   timeseries      timestamped numeric measurements
#   configs         key/value config maps with mixed scalar types
#   events          event log with levels, sources, payloads
#   array-numbers   flat array of numbers
#   array-strings   flat array of strings
#   array-booleans  flat array of booleans
#   array-mixed     array mixing numbers, strings, booleans, nulls
#   array-nested    deeply nested array
#   array-empty     empty array
#   object-flat     flat key/value object
#   object-nested   deeply nested object
#   string          a plain JSON string
#   number          a plain JSON number
#   boolean         a plain JSON boolean
#   null            JSON null
#
# Options:
#   -n, --count N   number of top-level items (default: 5)
#   -d, --depth N   nesting depth for nested types (default: 3)
#   -s, --seed N    integer seed for reproducible output (default: 42)
#   -p, --pretty    pretty-print via jq (if available)
#   -h, --help      show this help

set -euo pipefail

TYPE="repos"
COUNT=5
DEPTH=3
SEED=42
PRETTY=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        -n|--count)  COUNT="$2"; shift 2 ;;
        -d|--depth)  DEPTH="$2"; shift 2 ;;
        -s|--seed)   SEED="$2";  shift 2 ;;
        -p|--pretty) PRETTY=1;   shift   ;;
        -h|--help)
            sed -n '2,/^set /p' "$0" | grep -v '^set ' | sed 's/^# \?//'
            exit 0 ;;
        -*) echo "Unknown option: $1" >&2; exit 1 ;;
        *)  TYPE="$1"; shift ;;
    esac
done

# ── PRNG — LCG, all state in _R ───────────────────────────────────────────────
# IMPORTANT: never call nxt or rnd_* inside $(...) — that creates a subshell
# and the state change is lost.  All rnd_* functions set global R; callers do:
#   rnd_int 100; local x=$R
#
_R=$SEED
_V=0        # last raw value (= _R)

nxt() {               # nxt N  →  sets _V to value in [0,N); no subshell needed
    local n="${1:-32768}"
    _R=$(( (_R * 1103515245 + 12345) & 0x7fffffff ))
    _V=$(( _R % n ))
}

rnd_int()  { nxt "${1:-1000}"; R=$_V; }
rnd_bool() { nxt 2; [[ $_V -eq 0 ]] && R="true" || R="false"; }

rnd_float() {
    nxt 100; local w=$_V
    nxt 100; local f; printf -v f "%02d" $_V
    R="${w}.${f}"
}

rnd_date() {
    nxt 5;  local y=$(( 2020 + _V ))
    nxt 12; local m; printf -v m "%02d" $(( 1 + _V ))
    nxt 28; local d; printf -v d "%02d" $(( 1 + _V ))
    R="\"${y}-${m}-${d}\""
}

# rnd_pick ARRAY_NAME — sets R (bash 3 compatible, no nameref, no subshell)
rnd_pick() {
    eval "_sz=\${#${1}[@]}"
    nxt "$_sz"
    eval "R=\${${1}[$_V]}"
}

js() { printf '"%s"' "${1//\"/\\\"}"; }  # minimal JSON string quoting

lc() { echo "$1" | tr '[:upper:]' '[:lower:]'; }  # lowercase (bash 3 safe)

# ── lookup tables ─────────────────────────────────────────────────────────────
LANGS=(Rust Go Python TypeScript Zig C Kotlin Swift Haskell Elixir OCaml Lua)
TOPICS=(networking cryptography parser cli tui database compiler runtime tooling sdk auth cache)
LICENSES=(MIT Apache-2.0 GPL-3.0 BSD-2-Clause MPL-2.0 ISC AGPL-3.0 Unlicense)
BRANCHES=(main master develop staging "release/v1" "release/v2" "feature/auth" "hotfix/sec")
COMMIT_PREFIXES=(fix refactor add remove update bump revert migrate rename extract)
COMMIT_TAILS=(
    "off-by-one in lexer"
    "deprecated API surface"
    "dependency versions"
    "integration test suite"
    "parser pipeline"
    "helper module"
    "config format"
    "internal types"
    "accidental breakage"
    "memory leak in allocator"
)

SENSOR_LOCS=(roof basement lab-a lab-b server-room hallway garage outdoor rooftop attic)

PKG_SCOPES=("@infra" "@auth" "@db" "@ui" "@core" "@utils" "@api" "@test" "@cli" "@net")
PKG_NAMES=(logger router cache metrics schema codec retry config queue validator serializer)
PKG_VERS=("0.1.0" "0.2.3" "1.0.0" "1.2.1" "2.0.0-beta" "3.1.4" "0.9.9" "4.0.1" "1.0.0-rc1")

DNS_TYPES=(A AAAA CNAME MX TXT NS PTR)
DNS_HOSTS=(web-01 web-02 api mail smtp cdn static proxy db cache ns1 ns2)

GENRES=(Drama Thriller "Sci-Fi" Comedy Horror Action Romance Documentary Animation Mystery)
MPAA=(G PG "PG-13" R "NC-17")
ACTORS=(
    "Meryl Streep" "Denzel Washington" "Cate Blanchett" "Joaquin Phoenix"
    "Tilda Swinton" "Daniel Day-Lewis" "Frances McDormand" "Anthony Hopkins"
    "Viola Davis" "Chiwetel Ejiofor" "Saoirse Ronan" "Oscar Isaac"
    "Lupita Nyongo" "Adam Driver" "Thomasin McKenzie" "Paul Mescal"
)

PROC_NAMES=(nginx postgres redis node python rustc cargo rg fd bat jq gh tmux vim)
PROC_STATES=(S R D Z I T)

EVENT_SOURCES=(kernel scheduler network fs audit hypervisor oom-killer cron watchdog)
EVENT_LEVELS=(DEBUG INFO NOTICE WARN ERROR CRIT)
EVENT_MSGS=(
    "interface link up"
    "segfault at address"
    "connection refused on port"
    "disk usage above threshold"
    "TLS handshake failed"
    "process spawned"
    "mount point added"
    "swap usage critical"
    "firewall rule matched"
    "certificate expiring in 7 days"
)

CFG_KEYS=(max_connections timeout_ms retry_limit pool_size log_level cache_ttl batch_size worker_threads queue_depth flush_interval)

TITLE_ADJ=(Eternal Dark Lost Silent Broken Golden Iron Hollow Neon Crimson Pale Burning)
TITLE_NOUN=(Sky Storm Mind Gate Code Path Fall Rise Tide Wave Drift Signal)

# ── generators ────────────────────────────────────────────────────────────────

gen_repos() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick LANGS;    local lang=$R
        rnd_pick TOPICS;   local topic=$R
        rnd_pick LICENSES; local lic=$R
        rnd_pick BRANCHES; local branch=$R
        rnd_pick COMMIT_PREFIXES; local cpfx=$R
        rnd_pick COMMIT_TAILS;    local ctail=$R
        rnd_int 20000; local stars=$R
        rnd_int 3000;  local forks=$R
        rnd_int 500;   local issues=$R
        rnd_bool; local archived=$R
        rnd_bool; local is_fork=$R
        nxt 65536; local h1; printf -v h1 "%04x" $_V
        nxt 65536; local h2; printf -v h2 "%04x" $_V
        nxt 65536; local h3; printf -v h3 "%04x" $_V
        local sha="${h1}${h2}${h3}"
        local name; name=$(lc "${topic}-${lang}")
        [[ $i -gt 1 ]] && out+=","
        out+="{\"id\":${i},\"name\":$(js "$name"),\"language\":$(js "$lang"),\"license\":$(js "$lic"),\"stars\":${stars},\"forks\":${forks},\"open_issues\":${issues},\"archived\":${archived},\"fork\":${is_fork},\"default_branch\":$(js "$branch"),\"last_commit\":{\"sha\":$(js "$sha"),\"message\":$(js "${cpfx}: ${ctail}")}}"
    done
    echo "${out}]"
}

gen_sensors() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick SENSOR_LOCS; local loc=$R
        local sid; printf -v sid "SNS-%04d" $i
        nxt 700; local temp; temp=$(awk -v v=$_V 'BEGIN{printf "%.2f",(v/10.0)-10}')
        nxt 1000; local hum; hum=$(awk -v v=$_V 'BEGIN{printf "%.1f",v/10.0}')
        nxt 2000; local press=$(( 900 + _V ))
        rnd_bool; local online=$R
        nxt 100;  local bat=$_V
        local ts=$(( 1700000000 + i * 60 ))
        [[ $i -gt 1 ]] && out+=","
        out+="{\"sensor_id\":$(js "$sid"),\"location\":$(js "$loc"),\"timestamp\":${ts},\"online\":${online},\"battery_pct\":${bat},\"readings\":{\"temperature_c\":${temp},\"humidity_pct\":${hum},\"pressure_hpa\":${press}}}"
    done
    echo "${out}]"
}

gen_packages() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick PKG_SCOPES; local scope=$R
        rnd_pick PKG_NAMES;  local pname=$R
        rnd_pick PKG_VERS;   local ver=$R
        rnd_bool; local private=$R
        nxt 5; local ndeps=$_V
        local deps="["
        for (( d=0; d<ndeps; d++ )); do
            rnd_pick PKG_SCOPES; local ds=$R
            rnd_pick PKG_NAMES;  local dn=$R
            rnd_pick PKG_VERS;   local dv=$R
            rnd_bool; local dev=$R
            [[ $d -gt 0 ]] && deps+=","
            deps+="{\"name\":$(js "${ds}/${dn}"),\"version\":$(js "$dv"),\"dev\":${dev}}"
        done
        deps+="]"
        nxt 9999; local dl=$_V
        nxt 500;  local sz=$(( 10 + _V ))
        [[ $i -gt 1 ]] && out+=","
        out+="{\"name\":$(js "${scope}/${pname}"),\"version\":$(js "$ver"),\"private\":${private},\"weekly_downloads\":${dl},\"size_kb\":${sz},\"dependencies\":${deps}}"
    done
    echo "${out}]"
}

gen_dns() {
    local domain="example.internal"
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick DNS_TYPES; local dtype=$R
        rnd_pick DNS_HOSTS; local host=$R
        nxt 3600; local ttl=$(( 60 + _V ))
        local rdata
        case "$dtype" in
            A)
                nxt 256; local o1=$_V; nxt 256; local o2=$_V
                nxt 256; local o3=$_V; nxt 256; local o4=$_V
                rdata=$(js "${o1}.${o2}.${o3}.${o4}") ;;
            AAAA)
                nxt 65536; rdata=$(js "2001:db8:$(printf '%04x' $_V)::1") ;;
            CNAME)
                rnd_pick DNS_HOSTS; rdata=$(js "${R}.${domain}.") ;;
            MX)
                nxt 50; rdata=$(js "$(( 10 + _V )) smtp.${domain}.") ;;
            TXT)
                rdata=$(js "v=spf1 include:${domain} ~all") ;;
            NS)
                nxt 4; rdata=$(js "ns$(( 1 + _V )).${domain}.") ;;
            PTR)
                rdata=$(js "${host}.${domain}.") ;;
        esac
        [[ $i -gt 1 ]] && out+=","
        out+="{\"name\":$(js "${host}.${domain}"),\"type\":$(js "$dtype"),\"ttl\":${ttl},\"rdata\":${rdata}}"
    done
    echo "${out}]"
}

gen_movies() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick TITLE_ADJ;  local w1=$R
        rnd_pick TITLE_NOUN; local w2=$R
        nxt 45; local year=$(( 1970 + _V ))
        nxt 90; local runtime=$(( 70 + _V ))
        rnd_pick GENRES; local g1=$R
        rnd_pick GENRES; local g2=$R
        rnd_pick MPAA;   local rating=$R
        nxt 90; local score; score=$(awk -v v=$_V 'BEGIN{printf "%.1f",1+v/10.0}')
        nxt 100000; local votes=$(( 1000 + _V ))
        rnd_pick ACTORS; local a1=$R
        rnd_pick ACTORS; local a2=$R
        rnd_pick ACTORS; local dir=$R
        [[ $i -gt 1 ]] && out+=","
        out+="{\"id\":${i},\"title\":$(js "$w1 $w2"),\"year\":${year},\"runtime_min\":${runtime},\"rating\":$(js "$rating"),\"genres\":[$(js "$g1"),$(js "$g2")],\"score\":${score},\"votes\":${votes},\"director\":$(js "$dir"),\"cast\":[$(js "$a1"),$(js "$a2")]}"
    done
    echo "${out}]"
}

gen_processes() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick PROC_NAMES;  local pname=$R
        rnd_pick PROC_STATES; local pstate=$R
        nxt 65535; local pid=$(( 100 + _V ))
        nxt 999;   local ppid=$(( 1 + _V ))
        nxt 1000;  local cpu; cpu=$(awk -v v=$_V 'BEGIN{printf "%.1f",v/10.0}')
        nxt 32768; local rss=$(( 512 + _V ))
        nxt 99;    local threads=$(( 1 + _V ))
        nxt 40;    local nice=$(( -20 + _V ))
        nxt 86400; local uptime=$_V
        rnd_bool;  local daemon=$R
        [[ $i -gt 1 ]] && out+=","
        out+="{\"pid\":${pid},\"ppid\":${ppid},\"name\":$(js "$pname"),\"state\":$(js "$pstate"),\"cpu_pct\":${cpu},\"rss_kb\":${rss},\"threads\":${threads},\"nice\":${nice},\"uptime_sec\":${uptime},\"daemon\":${daemon}}"
    done
    echo "${out}]"
}

gen_matrix() {
    local cols=$(( DEPTH + 2 ))
    local out="["
    for (( r=0; r<COUNT; r++ )); do
        [[ $r -gt 0 ]] && out+=","
        out+="["
        for (( c=0; c<cols; c++ )); do
            rnd_float
            [[ $c -gt 0 ]] && out+=","
            out+="$R"
        done
        out+="]"
    done
    echo "${out}]"
}

gen_timeseries() {
    local out="["
    rnd_float; local base=$R
    for (( i=0; i<COUNT; i++ )); do
        nxt 200; local noise; noise=$(awk -v v=$_V 'BEGIN{printf "%.4f",(v-100)/500.0}')
        local val; val=$(awk -v b="$base" -v n="$noise" 'BEGIN{printf "%.4f",b+n}')
        rnd_bool; local anomaly=$R
        nxt 5;   local qual=$(( 1 + _V ))
        [[ $i -gt 0 ]] && out+=","
        out+="{\"t\":$(( 1700000000 + i * 30 )),\"v\":${val},\"quality\":${qual},\"anomaly\":${anomaly}}"
    done
    echo "${out}]"
}

gen_configs() {
    local n=$(( COUNT < ${#CFG_KEYS[@]} ? COUNT : ${#CFG_KEYS[@]} ))
    local out="{"
    for (( i=0; i<n; i++ )); do
        local key="${CFG_KEYS[$i]}"
        [[ $i -gt 0 ]] && out+=","
        nxt 4
        case $_V in
            0) nxt 10000; out+="$(js "$key"):$_V" ;;
            1) rnd_pick PROC_NAMES; out+="$(js "$key"):$(js "$R")" ;;
            2) rnd_bool; out+="$(js "$key"):$R" ;;
            3) rnd_float; out+="$(js "$key"):$R" ;;
        esac
    done
    echo "${out}}"
}

gen_events() {
    local out="["
    for (( i=1; i<=COUNT; i++ )); do
        rnd_pick EVENT_SOURCES; local src=$R
        rnd_pick EVENT_LEVELS;  local lvl=$R
        rnd_pick EVENT_MSGS;    local msg=$R
        nxt 9000; local code=$(( 1000 + _V ))
        nxt 65535; local pid=$(( 100 + _V ))
        local extra=""
        if [[ "$lvl" == "ERROR" || "$lvl" == "CRIT" ]]; then
            nxt 255; extra=",\"errno\":$_V,\"fatal\":true"
        fi
        [[ $i -gt 1 ]] && out+=","
        out+="{\"seq\":${i},\"timestamp\":$(( 1700000000 + i * 13 )),\"level\":$(js "$lvl"),\"source\":$(js "$src"),\"message\":$(js "$msg"),\"code\":${code},\"pid\":${pid}${extra}}"
    done
    echo "${out}]"
}

gen_array_numbers() {
    local out="["
    for (( i=0; i<COUNT; i++ )); do
        rnd_float; [[ $i -gt 0 ]] && out+=","
        out+="$R"
    done; echo "${out}]"
}

gen_array_strings() {
    local words=(alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima)
    local out="["
    for (( i=0; i<COUNT; i++ )); do
        rnd_pick words; [[ $i -gt 0 ]] && out+=","
        out+="$(js "${R}_${i}")"
    done; echo "${out}]"
}

gen_array_booleans() {
    local out="["
    for (( i=0; i<COUNT; i++ )); do
        rnd_bool; [[ $i -gt 0 ]] && out+=","
        out+="$R"
    done; echo "${out}]"
}

gen_array_mixed() {
    local strs=(alpha beta gamma delta epsilon zeta eta theta)
    local out="["
    for (( i=0; i<COUNT; i++ )); do
        [[ $i -gt 0 ]] && out+=","
        nxt 5
        case $_V in
            0) rnd_int 500;   out+="$R" ;;
            1) rnd_float;     out+="$R" ;;
            2) rnd_pick strs; out+="$(js "$R")" ;;
            3) rnd_bool;      out+="$R" ;;
            4) out+="null" ;;
        esac
    done; echo "${out}]"
}

gen_array_nested() {
    local d=$(( DEPTH > 5 ? 5 : DEPTH ))
    local out="["
    for (( i=0; i<COUNT; i++ )); do
        nxt 100; local leaf=$_V
        local part="$leaf"
        for (( lv=1; lv<d; lv++ )); do
            nxt 100; local x=$_V
            nxt 100; local y=$_V
            part="[$x,$part,$y]"
        done
        [[ $i -gt 0 ]] && out+=","
        out+="$part"
    done; echo "${out}]"
}

gen_array_empty() { echo "[]"; }

gen_object_flat() {
    local keys=(host port timeout retries enabled verbose debug level threshold ratio)
    local n=$(( COUNT < ${#keys[@]} ? COUNT : ${#keys[@]} ))
    local out="{"
    for (( i=0; i<n; i++ )); do
        [[ $i -gt 0 ]] && out+=","
        nxt 4
        case $_V in
            0) nxt 65535; out+="$(js "${keys[$i]}"):$_V" ;;
            1) rnd_pick LANGS; out+="$(js "${keys[$i]}"):$(js "$R")" ;;
            2) rnd_bool;       out+="$(js "${keys[$i]}"):$R" ;;
            3) rnd_float;      out+="$(js "${keys[$i]}"):$R" ;;
        esac
    done; echo "${out}}"
}

gen_object_nested() {
    local d=$(( DEPTH > 6 ? 6 : DEPTH ))
    rnd_pick LANGS; local lang=$R
    nxt 100; local val=$_V
    local part="{\"lang\":$(js "$lang"),\"value\":${val}}"
    for (( lv=2; lv<=d; lv++ )); do
        rnd_bool; local flag=$R
        rnd_pick TOPICS; local label=$R
        part="{\"depth\":${lv},\"label\":$(js "$label"),\"inner\":${part},\"active\":${flag}}"
    done; echo "$part"
}

gen_string()  {
    local w=(alpha bravo charlie delta epsilon)
    rnd_pick w; js "$R"
}
gen_number()  { rnd_float; echo "$R"; }
gen_boolean() { rnd_bool;  echo "$R"; }
gen_null()    { echo "null"; }

# ── dispatch ──────────────────────────────────────────────────────────────────
case "$TYPE" in
    repos)          OUTPUT=$(gen_repos)          ;;
    sensors)        OUTPUT=$(gen_sensors)        ;;
    packages)       OUTPUT=$(gen_packages)       ;;
    dns)            OUTPUT=$(gen_dns)            ;;
    movies)         OUTPUT=$(gen_movies)         ;;
    processes)      OUTPUT=$(gen_processes)      ;;
    matrix)         OUTPUT=$(gen_matrix)         ;;
    timeseries)     OUTPUT=$(gen_timeseries)     ;;
    configs)        OUTPUT=$(gen_configs)        ;;
    events)         OUTPUT=$(gen_events)         ;;
    array-numbers)  OUTPUT=$(gen_array_numbers)  ;;
    array-strings)  OUTPUT=$(gen_array_strings)  ;;
    array-booleans) OUTPUT=$(gen_array_booleans) ;;
    array-mixed)    OUTPUT=$(gen_array_mixed)    ;;
    array-nested)   OUTPUT=$(gen_array_nested)   ;;
    array-empty)    OUTPUT=$(gen_array_empty)    ;;
    object-flat)    OUTPUT=$(gen_object_flat)    ;;
    object-nested)  OUTPUT=$(gen_object_nested)  ;;
    string)         OUTPUT=$(gen_string)         ;;
    number)         OUTPUT=$(gen_number)         ;;
    boolean)        OUTPUT=$(gen_boolean)        ;;
    null)           OUTPUT=$(gen_null)           ;;
    *)
        echo "Unknown type: '$TYPE'. Run with --help for available types." >&2
        exit 1 ;;
esac

if [[ $PRETTY -eq 1 ]] && command -v jq &>/dev/null; then
    echo "$OUTPUT" | jq .
else
    echo "$OUTPUT"
fi
