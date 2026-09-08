name := 'cosmic-mini-taskmanager'
export APPID := 'io.github.kai_j_g.CosmicMiniTaskManager'

rootdir := env_var_or_default("DESTDIR", "")
prefix := env_var_or_default("PREFIX", if rootdir != "" { "/usr" } else { env_var('HOME') / '.local' })
base_dir := rootdir / prefix
bin_dir := base_dir / 'bin'
app_dir := base_dir / 'share' / 'applications'
metainfo_dir := base_dir / 'share' / 'metainfo'
icon_dir := base_dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps'

bin_src := 'target' / 'release' / name

default: build-release

build-release:
    cargo build --release

build-debug:
    cargo build

run:
    RUST_BACKTRACE=1 cargo run --release

install: build-release
    install -Dm0755 {{bin_src}} {{bin_dir}}/{{name}}
    install -Dm0644 data/{{APPID}}.desktop {{app_dir}}/{{APPID}}.desktop
    @if [ -z "{{rootdir}}" ] && [ "{{prefix}}" != "/usr" ]; then \
        sed -i 's|^Exec=.*|Exec={{bin_dir}}/{{name}}|' {{app_dir}}/{{APPID}}.desktop; \
    fi
    install -Dm0644 data/{{APPID}}.metainfo.xml {{metainfo_dir}}/{{APPID}}.metainfo.xml
    install -Dm0644 data/icons/{{APPID}}-symbolic.svg {{icon_dir}}/{{APPID}}-symbolic.svg
    install -Dm0644 data/icons/{{APPID}}.svg {{icon_dir}}/{{APPID}}.svg
    @echo "Installed Cosmic Mini TaskManager to {{bin_dir}}/{{name}}"
    @echo "Add it via: COSMIC Settings -> Desktop -> Panel -> Applets -> Add Applet"

uninstall:
    rm -f {{bin_dir}}/{{name}}
    rm -f {{app_dir}}/{{APPID}}.desktop
    rm -f {{metainfo_dir}}/{{APPID}}.metainfo.xml
    rm -f {{icon_dir}}/{{APPID}}-symbolic.svg
    rm -f {{icon_dir}}/{{APPID}}.svg
    @echo "Uninstalled Cosmic Mini TaskManager from {{prefix}}"

check:
    cargo check
    cargo test
