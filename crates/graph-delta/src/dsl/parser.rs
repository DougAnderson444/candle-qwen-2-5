//! Parser using the pest
use super::ast::*;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::collections::HashMap; // Added missing import for HashMap

#[derive(Parser)]
#[grammar = "dsl/graphdsl.pest"]
pub struct DslParser;

pub fn parse_dsl(input: &str) -> Result<Vec<DslCommand>, pest::error::Error<Rule>> {
    let mut cmds = vec![];
    let file = DslParser::parse(Rule::file, input)?.next().unwrap();

    for stmt in file.into_inner() {
        match stmt.as_rule() {
            // Node Commands
            Rule::node_add_cmd => cmds.push(parse_node_add_cmd(stmt)),
            Rule::node_update_cmd => cmds.push(parse_node_update_cmd(stmt)),
            Rule::node_delete_cmd => cmds.push(parse_node_delete_cmd(stmt)),

            // Edge Commands
            Rule::edge_add_cmd => cmds.push(parse_edge_add_cmd(stmt)),
            Rule::edge_update_cmd => cmds.push(parse_edge_update_cmd(stmt)),
            Rule::edge_delete_cmd => cmds.push(parse_edge_delete_cmd(stmt)),

            // Subgraph Commands
            Rule::subgraph_add_cmd => cmds.push(parse_subgraph_add_cmd(stmt)),
            Rule::subgraph_update_cmd => cmds.push(parse_subgraph_update_cmd(stmt)),
            Rule::subgraph_move_cmd => cmds.push(parse_subgraph_move_cmd(stmt)),
            Rule::subgraph_delete_cmd => cmds.push(parse_subgraph_delete_cmd(stmt)),

            // Global Commands
            Rule::graph_set_cmd => cmds.push(parse_graph_set_cmd(stmt)),
            Rule::node_defaults_cmd => cmds.push(parse_node_defaults_cmd(stmt)),
            Rule::edge_defaults_cmd => cmds.push(parse_edge_defaults_cmd(stmt)),

            // Rank Commands
            Rule::rank_same_cmd => cmds.push(parse_rank_same_cmd(stmt)),
            Rule::rank_min_cmd => cmds.push(parse_rank_min_cmd(stmt)),
            Rule::rank_max_cmd => cmds.push(parse_rank_max_cmd(stmt)),
            _ => {} // Ignore WHITESPACE or NEWLINE rules
        }
    }

    Ok(cmds)
}

fn parse_attrs(pair: Pair<Rule>) -> Attrs {
    pair.into_inner()
        .map(|a| {
            let mut i = a.into_inner();
            let key = i.next().unwrap().as_str().to_string();
            let val = i.next().unwrap().as_str().trim().to_string();
            (key, val)
        })
        .collect::<HashMap<_, _>>()
}

fn parse_ident_list(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner().map(|p| p.as_str().to_string()).collect()
}

// --- Node Command Parsers ---
fn parse_node_add_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Node(NodeCmd::Add { id, attrs })
}

fn parse_node_update_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Node(NodeCmd::Update { id, attrs })
}

fn parse_node_delete_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    DslCommand::Node(NodeCmd::Delete { id })
}

// --- Edge Command Parsers ---
fn parse_edge_add_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let from = inner.next().unwrap().as_str().to_string();
    let to = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Edge(EdgeCmd::Add { from, to, attrs })
}

fn parse_edge_update_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let from = inner.next().unwrap().as_str().to_string();
    let to = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Edge(EdgeCmd::Update { from, to, attrs })
}

fn parse_edge_delete_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let from = inner.next().unwrap().as_str().to_string();
    let to = inner.next().unwrap().as_str().to_string();
    DslCommand::Edge(EdgeCmd::Delete { from, to })
}

// --- Subgraph Command Parsers ---
fn parse_subgraph_add_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Cluster(ClusterCmd::Add { id, attrs })
}

fn parse_subgraph_update_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let attrs = inner.next().map(parse_attrs).unwrap_or_default();
    DslCommand::Cluster(ClusterCmd::Update { id, attrs })
}

fn parse_subgraph_move_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let node = inner.next().unwrap().as_str().to_string();
    let cluster = inner.next().unwrap().as_str().to_string();
    DslCommand::Cluster(ClusterCmd::Move { node, cluster })
}

fn parse_subgraph_delete_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    DslCommand::Cluster(ClusterCmd::Delete { id })
}

// --- Global Command Parsers ---
fn parse_graph_set_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let attrs = parse_attrs(inner.next().unwrap());
    DslCommand::Global(GlobalCmd::Set(attrs))
}

fn parse_node_defaults_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let attrs = parse_attrs(inner.next().unwrap());
    DslCommand::Global(GlobalCmd::NodeDefaults(attrs))
}

fn parse_edge_defaults_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let attrs = parse_attrs(inner.next().unwrap());
    DslCommand::Global(GlobalCmd::EdgeDefaults(attrs))
}

// --- Rank Command Parsers ---
fn parse_rank_same_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let list = parse_ident_list(inner.next().unwrap());
    DslCommand::Rank(RankCmd::Same(list))
}

fn parse_rank_min_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let list = parse_ident_list(inner.next().unwrap());
    DslCommand::Rank(RankCmd::Min(list))
}

fn parse_rank_max_cmd(pair: Pair<Rule>) -> DslCommand {
    let mut inner = pair.into_inner();
    let list = parse_ident_list(inner.next().unwrap());
    DslCommand::Rank(RankCmd::Max(list))
}
