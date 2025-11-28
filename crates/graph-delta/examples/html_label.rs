use graph_delta::parser::{DotParser, Rule};
use pest::Parser;

fn main() {
    let dot_input = include_str!("../tests/fixtures/kitchen_sink.dot");

    match DotParser::parse(Rule::dotfile, dot_input) {
        Ok(mut pairs) => {
            println!("✓ Successfully parsed DOT file with HTML labels and nested clusters!\n");

            // Walk through the parse tree
            for pair in pairs.next().unwrap().into_inner() {
                print_structure(pair, 0);
            }
        }
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
        }
    }
}

fn print_structure(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let rule = pair.as_rule();

    match rule {
        Rule::dotgraph => {
            println!(
                "{}DIGRAPH: {}",
                indent_str,
                pair.as_str().lines().next().unwrap_or("")
            );
            for inner in pair.into_inner() {
                print_structure(inner, indent + 1);
            }
        }
        Rule::stmt_list => {
            println!("{}Statement List:", indent_str);
            for inner in pair.into_inner() {
                print_structure(inner, indent + 1);
            }
        }
        Rule::attr_stmt => {
            println!("{}ATTR_STMT (graph/node/edge attributes):", indent_str);
            for inner in pair.into_inner() {
                print_structure(inner, indent + 1);
            }
        }
        Rule::subgraph => {
            let name = pair
                .clone()
                .into_inner()
                .find(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str())
                .unwrap_or("(anonymous)");
            println!("{}SUBGRAPH: {}", indent_str, name);
            for inner in pair.into_inner() {
                print_structure(inner, indent + 1);
            }
        }
        Rule::node_stmt => {
            let node_name = pair
                .clone()
                .into_inner()
                .next()
                .and_then(|p| p.into_inner().next())
                .map(|p| p.as_str())
                .unwrap_or("?");
            println!("{}NODE: {}", indent_str, node_name);
        }
        Rule::edge_stmt => {
            println!("{}EDGE", indent_str);
        }
        Rule::html => {
            let preview = pair.as_str().chars().take(50).collect::<String>();
            println!("{}HTML: {}...", indent_str, preview);
        }
        Rule::id_eq => {
            println!("{}ID_EQ: {}", indent_str, pair.as_str());
        }
        _ => {
            // Recurse into children without printing
            for inner in pair.into_inner() {
                print_structure(inner, indent);
            }
        }
    }
}
