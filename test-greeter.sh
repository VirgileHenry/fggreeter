if ! command -v fakegreet >/dev/null; then
    echo "fakegreet not found, install it with:" >&2
    echo "  cargo install --git https://github.com/kennylevinsen/greetd fakegreet" >&2
    exit 1
fi

cargo build --features dev && BEVY_ASSET_ROOT="$PWD" fakegreet "target/debug/fggreeter --user user --command start-hyprland"
