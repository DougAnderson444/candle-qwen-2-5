use graph_delta::parser::{parse_dot_to_chunks, DotParser};
use pest::Parser;

fn main() {
    let dot = r#"digraph Example { A [label="Node A"]; }"#;
    
    // First, let's see what the raw pest output looks like
    println!("=== Raw Pest Parse Tree ===");
    match DotParser::parse(pest::Parser::Rule::dotfile, dot) {
        Ok(pairs) => {
            for pair in pairs {
                print_pair(pair, 0);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n=== Chunks ===");
    match parse_dot_to_chunks(dot) {
        Ok(chunks) => {
            for chunk in chunks {
                println!("{:?}", chunk);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

fn print_pair(pair: pest::iterators::Pair<pest::Parser::Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}Rule::{:?} => \"{}\"", indent_str, pair.as_rule(), pair.as_str());
    for inner_pair in pair.into_inner() {
        print_pair(inner_pair, indent + 1);
    }
}
