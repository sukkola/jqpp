pub mod app;
pub mod completions;
pub mod config;
pub mod executor;
pub mod keymap;
pub mod ui;
pub mod widgets;

// Temporary debug - will remove
#[allow(dead_code)]
pub fn debug_contains_completions() {
    use crate::completions::json_context::get_completions;
    use serde_json::json;

    let input = json!({
        "orders": [
            {
                "customer": {
                    "customer_email": "alice@example.com",
                    "customer_id": "CUST-42",
                    "customer_name": "Alice"
                },
                "items": [],
                "order_date": "2024-01-10",
                "order_id": "ORD-001",
                "order_status": "shipped",
                "totals": {}
            }
        ]
    });

    let orders = input["orders"].clone();
    let query = " contains([{customer: {customer_email: \"alice@example.com\", ";

    eprintln!("DEBUG completions for: {:?}", query);
    let completions = get_completions(query, &orders);
    for c in &completions {
        eprintln!("  label={:?} detail={:?}", c.label, c.detail);
    }
}
