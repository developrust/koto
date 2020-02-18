use pest::{error::Error, Parser, Span};
use std::rc::Rc;

#[derive(Parser)]
#[grammar = "holz.pest"]
struct HolzParser;

#[derive(Clone, Debug)]
pub struct AstNode {
    pub node: Node,
    pub start_pos: Position,
    pub end_pos: Position,
}

impl AstNode {
    pub fn new(span: Span, node: Node) -> Self {
        let line_col = span.start_pos().line_col();
        let start_pos = Position {
            line: line_col.0,
            column: line_col.1,
        };
        let line_col = span.end_pos().line_col();
        let end_pos = Position {
            line: line_col.0,
            column: line_col.1,
        };
        Self {
            node,
            start_pos,
            end_pos,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Bool(bool),
    Number(f64),
    Str(Rc<String>),
    Array(Vec<AstNode>),
    Range {
        min: Box<AstNode>,
        max: Box<AstNode>,
        inclusive: bool,
    },
    Id(Rc<String>),
    Block(Vec<AstNode>),
    Function(Rc<Function>),
    Call {
        function: Rc<String>,
        args: Vec<AstNode>,
    },
    Index {
        id: String,
        expression: Box<AstNode>,
    },
    Assign {
        id: Rc<String>,
        expression: Box<AstNode>,
    },
    BinaryOp {
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
        op: Op,
    },
    If {
        condition: Box<AstNode>,
        then_node: Box<AstNode>,
        else_node: Option<Box<AstNode>>,
    },
    For(Rc<AstFor>),
}

#[derive(Clone, Debug)]
pub struct Block {}

#[derive(Clone, Debug)]
pub struct Function {
    pub args: Vec<Rc<String>>,
    pub body: Vec<AstNode>,
}

#[derive(Clone, Debug)]
pub struct AstFor {
    pub arg: Rc<String>,
    pub range: Box<AstNode>,
    pub condition: Option<Box<AstNode>>,
    pub body: Box<AstNode>,
}

#[derive(Clone, Debug)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

pub fn parse(source: &str) -> Result<Vec<AstNode>, Error<Rule>> {
    let parsed = HolzParser::parse(Rule::program, source)?;
    // dbg!(&parsed);

    let mut ast = vec![];
    for pair in parsed {
        match pair.as_rule() {
            Rule::block => {
                ast.push(build_ast_from_expression(pair).unwrap());
            }
            _ => {}
        }
    }

    Ok(ast)
}

fn build_ast_from_expression(pair: pest::iterators::Pair<Rule>) -> Option<AstNode> {
    // dbg!(&pair);
    use Node::*;
    let span = pair.as_span();
    match pair.as_rule() {
        Rule::push_indentation | Rule::indentation => None,
        Rule::expression | Rule::lhs_value | Rule::rhs_value => {
            build_ast_from_expression(pair.into_inner().next().unwrap())
        }
        Rule::block => {
            let inner = pair.into_inner();
            let block: Vec<AstNode> = inner
                .filter_map(|pair| build_ast_from_expression(pair))
                .collect();
            Some(AstNode::new(span, Block(block)))
        }
        Rule::boolean => Some(AstNode::new(span, Bool(pair.as_str().parse().unwrap()))),
        Rule::number => Some(AstNode::new(
            span,
            Node::Number(pair.as_str().parse().unwrap()),
        )),
        Rule::string => Some(AstNode::new(
            span,
            Node::Str(Rc::new(
                pair.into_inner().next().unwrap().as_str().to_string(),
            )),
        )),
        Rule::array => {
            let inner = pair.into_inner();
            let elements: Vec<AstNode> = inner
                .filter_map(|pair| build_ast_from_expression(pair))
                .collect();
            Some(AstNode::new(span, Node::Array(elements)))
        }
        Rule::range => {
            let mut inner = pair.into_inner();

            let min = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            let inclusive = inner.next().unwrap().as_str() == "..=";
            let max = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());

            Some(AstNode::new(
                span,
                Node::Range {
                    min,
                    inclusive,
                    max,
                },
            ))
        }
        Rule::index => {
            let mut inner = pair.into_inner();
            let id = inner.next().unwrap().as_str().to_string();
            let expression = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            Some(AstNode::new(span, Node::Index { id, expression }))
        }
        Rule::id => Some(AstNode::new(
            span,
            Node::Id(Rc::new(pair.as_str().to_string())),
        )),
        Rule::function => {
            let mut inner = pair.into_inner();
            let mut capture = inner.next().unwrap().into_inner();
            let args: Vec<Rc<String>> = capture
                .by_ref()
                .take_while(|pair| pair.as_str() != "->")
                .map(|pair| Rc::new(pair.as_str().to_string()))
                .collect();
            // collect function body
            let body: Vec<AstNode> = inner
                .filter_map(|pair| build_ast_from_expression(pair))
                .collect();
            Some(AstNode::new(
                span,
                Node::Function(Rc::new(self::Function { args, body })),
            ))
        }
        Rule::call => {
            let mut inner = pair.into_inner();
            let function = Rc::new(inner.next().unwrap().as_str().to_string());
            let args: Vec<AstNode> = inner
                .filter_map(|pair| build_ast_from_expression(pair))
                .collect();
            Some(AstNode::new(span, Node::Call { function, args }))
        }
        Rule::assignment => {
            let mut inner = pair.into_inner();
            let id = Rc::new(inner.next().unwrap().as_str().to_string());
            let expression = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            Some(AstNode::new(span, Node::Assign { id, expression }))
        }
        Rule::binary_op => {
            let mut inner = pair.into_inner();
            let lhs = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            let op = match inner.next().unwrap().as_str() {
                "+" => Op::Add,
                "-" => Op::Subtract,
                "*" => Op::Multiply,
                "/" => Op::Divide,
                "==" => Op::Equal,
                "!=" => Op::NotEqual,
                "<" => Op::LessThan,
                "<=" => Op::LessThanOrEqual,
                ">" => Op::GreaterThan,
                ">=" => Op::GreaterThanOrEqual,
                "and" => Op::And,
                "or" => Op::Or,
                unexpected => {
                    let error = format!("Unexpected binary operator: {}", unexpected);
                    unreachable!(error)
                }
            };
            let rhs = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            Some(AstNode::new(span, Node::BinaryOp { lhs, op, rhs }))
        }
        Rule::if_inline | Rule::if_block => {
            // dbg!(&pair);
            let mut inner = pair.into_inner();
            inner.next(); // if
            let condition = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            inner.next(); // then, or block start
            let then_node = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            let else_node = if inner.next().is_some() {
                Some(Box::new(
                    build_ast_from_expression(inner.next().unwrap()).unwrap(),
                ))
            } else {
                None
            };

            Some(AstNode::new(
                span,
                Node::If {
                    condition,
                    then_node,
                    else_node,
                },
            ))
        }
        Rule::for_block => {
            // dbg!(&pair);
            let mut inner = pair.into_inner();
            inner.next(); // for
            let arg = Rc::new(inner.next().unwrap().as_str().to_string());
            inner.next(); // in
            let range = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            let condition = if inner.peek().unwrap().as_rule() == Rule::if_keyword {
                inner.next();
                Some(Box::new(
                    build_ast_from_expression(inner.next().unwrap()).unwrap(),
                ))
            } else {
                None
            };
            inner.next(); // indentation
            let body = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            Some(AstNode::new(
                span,
                Node::For(Rc::new(AstFor {
                    arg,
                    range,
                    condition,
                    body,
                })),
            ))
        }
        Rule::for_inline => {
            // dbg!(&pair);
            let mut inner = pair.into_inner();
            let body = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            inner.next(); // for
            let arg = Rc::new(inner.next().unwrap().as_str().to_string());
            inner.next(); // in
            let range = Box::new(build_ast_from_expression(inner.next().unwrap()).unwrap());
            let condition = if inner.next().is_some() { // if
                Some(Box::new(
                    build_ast_from_expression(inner.next().unwrap()).unwrap(),
                ))
            } else {
                None
            };
            Some(AstNode::new(
                span,
                Node::For(Rc::new(AstFor {
                    arg,
                    range,
                    condition,
                    body,
                })),
            ))
        }
        unexpected => unreachable!("Unexpected expression: {:?}", unexpected),
    }
}
