# demo.tape — build first, then regenerate: cd demo && vhs demo.tape
# https://github.com/charmbracelet/vhs

Output demo.gif

Require kak

Set Shell bash
# Catppuccin Mocha's ANSI palette on a black background, so the scope
# backgrounds (rgb:181825 and friends) are actually visible.
Set Theme { "background": "#000000", "foreground": "#cdd6f4", "cursor": "#f5e0dc", "selection": "#585b70", "black": "#45475a", "red": "#f38ba8", "green": "#a6e3a1", "yellow": "#f9e2af", "blue": "#89b4fa", "magenta": "#f5c2e7", "cyan": "#94e2d5", "white": "#bac2de", "brightBlack": "#585b70", "brightRed": "#f38ba8", "brightGreen": "#a6e3a1", "brightYellow": "#f9e2af", "brightBlue": "#89b4fa", "brightMagenta": "#f5c2e7", "brightCyan": "#94e2d5", "brightWhite": "#a6adc8" }
Set FontSize 18
Set Width 1100
Set Height 640
Set Padding 10
Set Framerate 20
Set TypingSpeed 55ms

# Setup (not recorded): point kak at a throwaway config that sources this
# plugin and nothing else, so the demo does not depend on anyone's kakrc.
Hide
Type 'cd ..' Enter
Type 'export XDG_CONFIG_HOME=$(mktemp -d) && mkdir -p $XDG_CONFIG_HOME/kak' Enter
Type 'echo "set-option global startup_info_version 99999999" > $XDG_CONFIG_HOME/kak/kakrc' Enter
Type 'echo "source $PWD/rc/rainbow.kak" >> $XDG_CONFIG_HOME/kak/kakrc' Enter
Type 'clear' Enter
Show

# Plain buffer: brackets are the same color as everything else.
Type 'kak +52 src/brackets.rs' Enter
Sleep 2.5s

# Brackets light up, colored by nesting level. Note the b'(' b'[' b'{'
# char literals stay uncolored: the tokenizer skips them.
Type ':rainbow-enable-window' Enter
Sleep 3.5s

# Mode 1 (the default): the cursor's scope gets a background. Walk down
# through the nested scopes, pausing on each so the idle hook re-renders.
Type '12k'
Sleep 2s
Type '10j'
Sleep 2s
Type '8j'
Sleep 2s
Type '10j'
Sleep 2.5s

# Mode 2: every scope gets a background.
Type ':set global rainbow_mode 2' Enter
Sleep 4s
