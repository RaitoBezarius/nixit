use std::{io::{stdin, Read}, fmt::{Display, write}};
use clap::Parser;
use tree_sitter::Node;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    add_expr: Vec<String>,
    #[arg(long)]
    remove_expr: Vec<String>,
}

#[derive(Debug)]
struct Context {
    local_bindings: Vec<String>,
    with_contexts: Vec<String>
}

#[derive(Debug)]
struct ValueWithContext<'a> {
    context: Option<Node<'a>>,
    value: Node<'a>,
    original_contents: &'a [u8]
}

impl<'a> Display for ValueWithContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("context: {:?} - value: {}",
                self.context.map(|n| n.utf8_text(self.original_contents).unwrap()),
                self.value.utf8_text(self.original_contents).unwrap()
            ))
    }
}

struct ValueQuery<'a, 'tree> {
    query: tree_sitter::Query,
    cursor: tree_sitter::QueryCursor,
    root_node: Node<'tree>,
    original_contents: &'a [u8],
}

impl<'a: 'tree, 'tree> ValueQuery<'a, 'tree> {
    fn into_iter(&'a mut self) -> impl Iterator<Item = ValueWithContext<'tree>> + 'tree {
        self.cursor.captures(&self.query, self.root_node, self.original_contents)
        .flat_map(|(match_, _)| {
            println!("match: {:?}", match_);
            let env = self.query.capture_index_for_name("env")
                .and_then(|index| match_.captures.get(index as usize))
                .map(|capture| capture.node);

            let elements = self.query.capture_index_for_name("elements")
                .and_then(|index| match_.captures.get(index as usize))
                .map_or_else(|| vec![], |capture| vec![capture.node]);
            
            for capture in match_.captures {
                println!("\tcapture: {:?}", capture);
                println!("\tcaptured text: {:?}", capture.node.utf8_text(&self.original_contents));
            }

            let contents = self.original_contents;
            elements.into_iter().map(move |elt| ValueWithContext {
                context: env,
                value: elt,
                original_contents: contents
            })
        })
    }
}

enum NixValueTypes {
    List,
    Attrset
}

struct NixFile<'a> {
    contents: &'a [u8],
    tree: tree_sitter::Tree,
}

impl<'a> NixFile<'a> {
    fn select_value(&'a mut self, attrpath: &str, value_type: NixValueTypes) -> ValueQuery<'a, 'a> {
        // 1. TODO: expand attrpath into a normalized attrpath (i.e. rope of attrpath)
        let attrpath_sexpr = format!("(attrpath attr: (identifier) @identifier (#match @identifier \"{attrpath}\"))");
        // 2. build the query
        let expression = match value_type {
            NixValueTypes::List => "(list_expression . (_)* @elements)",
            NixValueTypes::Attrset => todo!()
        };

        let with_expressions = format!("(with_expression environment: (_) @env body: {})", expression); // (with x; with y; with z; ...)
        let let_expressions = ""; // (let x = y; z = t; u = v; w = x; in ...)

        let value_expression = format!("{}", with_expressions);
        let query = format!("(binding attrpath: {} expression: {})",
            attrpath_sexpr,
            value_expression
        );

        println!("query: {:#?}", query);

        ValueQuery {
            query: tree_sitter::Query::new(tree_sitter_nix::language(), &query).unwrap(),
            cursor: tree_sitter::QueryCursor::new(),
            root_node: self.tree.root_node(),
            original_contents: &self.contents
        }
    }
}

fn main() {
    let args = Args::parse();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_nix::language())
        .expect("Language incorrectly set");

    let mut contents = String::new();
    stdin().read_to_string(&mut contents).expect("Failed to read stdin");

    if let Some(tree) = parser.parse(contents.clone(), None) {
        println!("{:?}", tree.root_node().to_sexp());
        let mut file = NixFile {
            contents: contents.as_bytes(),
            tree
        };

        for value in file.select_value("maintainers", NixValueTypes::List).into_iter() {
            println!("{}", value);
        }
        // 2. Render add_expr
        // 3. Render remove_expr
    }
}
