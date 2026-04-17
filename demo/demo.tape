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

# --- Scene 1: Ghost suggestion — .orders is an array, [] ghost appears, Tab accepts ---
Type ".orders"
Sleep 700ms
# Ghost "[]" appears — accept with Tab
Tab
Sleep 500ms
# Now at ".orders[]" — type prefix to filter fields
Type ".order_s"
Sleep 500ms
# Dropdown shows "order_status" (only field starting with order_s)
Tab
Sleep 800ms
# Results: "shipped", "processing", "delivered"

# --- Clear ---
Set TypingSpeed 10ms
Backspace 40
Set TypingSpeed 90ms
Sleep 300ms

# --- Scene 2: Fuzzy completion — fragment matches non-prefix field ---
Type ".orders[] | .totals | ."
Sleep 500ms
# "grtot" fuzzy-finds grand_total even though it's not a prefix match
Type "grtot"
Sleep 600ms
# Dropdown shows ~grand_total
Tab
Sleep 800ms
# Result: 171.7, 73.5, 212.17

# --- Clear ---
Set TypingSpeed 10ms
Backspace 40
Set TypingSpeed 90ms
Sleep 300ms

# --- Scene 3: Param field completions — sort_by knows its arg is a field name ---
Type ".products[] | sort_by(."
Sleep 600ms
# Dropdown shows sku, name, category, price — fields of product objects
Type "pr"
Sleep 400ms
# Filtered to "price"
Tab
Sleep 800ms
# Products sorted by price: Mini Widget 15, Premium Widget 22.50, …

# --- Clear ---
Set TypingSpeed 10ms
Backspace 40
Set TypingSpeed 90ms
Sleep 300ms

# --- Scene 4: Deep path — prefix filters nested fields ---
Type ".orders[].customer."
Sleep 500ms
# customer_id, customer_name, customer_email appear
Type "customer_n"
Sleep 500ms
# Filtered to customer_name
Tab
Sleep 800ms
# Results: "Alice Korhonen", "Mikko Virtanen", "Sara Mäkinen"

# --- Clear ---
Set TypingSpeed 10ms
Backspace 40
Set TypingSpeed 90ms
Sleep 300ms

# --- Scene 5: Pane scrolling ---
Type ".orders[] | .items[]"
Sleep 600ms
# Shift+Tab to focus right output pane
Shift+Tab
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
PageDown
Sleep 400ms
PageUp
Sleep 300ms
# Back to query bar
Tab
Sleep 300ms

# --- Clear ---
Set TypingSpeed 10ms
Backspace 30
Set TypingSpeed 90ms
Sleep 300ms

# --- Scene 6: Builtin select ---
Type ".orders[] | sel"
Sleep 600ms
# Dropdown shows select builtin
Tab
Sleep 700ms
# Query: .orders[] | select(. != null)

# Exit
Ctrl+C
Sleep 500ms
