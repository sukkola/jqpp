Output demo/demo.gif

Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 90ms
Set Theme "Catppuccin Frappe"

# Launch jqpp with demo data
Type "jqpp demo/demo.json"
Enter
Sleep 1.5s

# --- Scene 1: Top-level dot, type "st" to filter store_* fields, navigate and accept ---
Type "."
Sleep 700ms
# Suggestions: store_id, store_name, store_region, metadata, orders, products
Type "st"
Sleep 700ms
# Filtered to: store_id, store_name, store_region
Down
Sleep 350ms
# On store_name
Tab
Sleep 1s
# Result: "Nordic Widgets"

# --- Clear ---
Backspace 50
Sleep 400ms

# --- Scene 2: Drill into array, filter order_ fields, Up/Down navigate, accept ---
Type ".orders[] | ."
Sleep 700ms
# Suggestions: order_id, order_status, order_date, customer, items, totals
Type "order"
Sleep 700ms
# Filtered to: order_id, order_status, order_date
Down
Sleep 350ms
# On order_status
Down
Sleep 350ms
# On order_date
Up
Sleep 350ms
# Back on order_status
Tab
Sleep 1s
# Results: "shipped", "processing", "delivered"

# --- Clear ---
Backspace 50
Sleep 400ms

# --- Scene 3: Deep dot-path, filter to grand_total ---
Type ".orders[].totals.grand"
Sleep 700ms
# Filtered to: grand_total
Tab
Sleep 1s
# Results: 171.7, 73.5, 212.17

# --- Clear ---
Backspace 50
Sleep 400ms

# --- Scene 4: Pipe + builtin select completion ---
Type ".orders[] | sel"
Sleep 700ms
# Suggestions: select (builtin)
Tab
Sleep 1s
# Query: .orders[] | select(. != null) — shows all 3 orders

# Exit
Ctrl+C
Sleep 500ms
