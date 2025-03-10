mod binary_op;
mod is_null;
mod nested;
mod unary_op;

pub mod aggregate;
pub mod between;
pub mod case;
pub mod extract;
pub mod function;
pub mod in_list;

pub use case::case;
pub use nested::nested;

use {
    crate::{
        ast::{Aggregate, AstLiteral, BinaryOperator, DateTimeField, Expr, UnaryOperator},
        parse_sql::parse_expr,
        result::{Error, Result},
        translate::translate_expr,
    },
    aggregate::AggregateNode,
    bigdecimal::BigDecimal,
    function::FunctionNode,
};

#[derive(Clone)]
pub enum ExprNode {
    Expr(Expr),
    SqlExpr(String),
    Identifier(String),
    CompoundIdentifier(Vec<String>),
    Between {
        expr: Box<ExprNode>,
        negated: bool,
        low: Box<ExprNode>,
        high: Box<ExprNode>,
    },
    BinaryOp {
        left: Box<ExprNode>,
        op: BinaryOperator,
        right: Box<ExprNode>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<ExprNode>,
    },
    Extract {
        field: DateTimeField,
        expr: Box<ExprNode>,
    },
    IsNull(Box<ExprNode>),
    IsNotNull(Box<ExprNode>),
    InList {
        expr: Box<ExprNode>,
        list: Vec<ExprNode>,
        negated: bool,
    },
    Case {
        operand: Option<Box<ExprNode>>,
        when_then: Vec<(ExprNode, ExprNode)>,
        else_result: Option<Box<ExprNode>>,
    },
    Nested(Box<ExprNode>),
    Function(Box<FunctionNode>),
    Aggregate(Box<AggregateNode>),
}

impl TryFrom<ExprNode> for Expr {
    type Error = Error;

    fn try_from(expr_node: ExprNode) -> Result<Self> {
        match expr_node {
            ExprNode::Expr(expr) => Ok(expr),
            ExprNode::SqlExpr(expr) => {
                let expr = parse_expr(expr)?;

                translate_expr(&expr)
            }
            ExprNode::Identifier(ident) => Ok(Expr::Identifier(ident)),
            ExprNode::CompoundIdentifier(idents) => Ok(Expr::CompoundIdentifier(idents)),
            ExprNode::Between {
                expr,
                negated,
                low,
                high,
            } => {
                let expr = Expr::try_from(*expr).map(Box::new)?;
                let low = Expr::try_from(*low).map(Box::new)?;
                let high = Expr::try_from(*high).map(Box::new)?;

                Ok(Expr::Between {
                    expr,
                    negated,
                    low,
                    high,
                })
            }
            ExprNode::BinaryOp { left, op, right } => {
                let left = Expr::try_from(*left).map(Box::new)?;
                let right = Expr::try_from(*right).map(Box::new)?;

                Ok(Expr::BinaryOp { left, op, right })
            }
            ExprNode::UnaryOp { op, expr } => {
                let expr = Expr::try_from(*expr).map(Box::new)?;
                Ok(Expr::UnaryOp { op, expr })
            }
            ExprNode::Extract { field, expr } => {
                let expr = Expr::try_from(*expr).map(Box::new)?;
                Ok(Expr::Extract { field, expr })
            }
            ExprNode::IsNull(expr) => Expr::try_from(*expr).map(Box::new).map(Expr::IsNull),
            ExprNode::IsNotNull(expr) => Expr::try_from(*expr).map(Box::new).map(Expr::IsNotNull),
            ExprNode::InList {
                expr,
                list,
                negated,
            } => {
                let expr = Expr::try_from(*expr).map(Box::new)?;
                let list = list
                    .into_iter()
                    .map(Expr::try_from)
                    .collect::<Result<Vec<_>>>()?;

                Ok(Expr::InList {
                    expr,
                    list,
                    negated,
                })
            }
            ExprNode::Nested(expr) => Expr::try_from(*expr).map(Box::new).map(Expr::Nested),
            ExprNode::Function(func_expr) => Expr::try_from(*func_expr),
            ExprNode::Aggregate(aggr_expr) => Aggregate::try_from(*aggr_expr)
                .map(Box::new)
                .map(Expr::Aggregate),
            ExprNode::Case {
                operand,
                when_then,
                else_result,
            } => {
                let operand_expr = operand.map(|expr| Expr::try_from(*expr).map(Box::new).unwrap());
                let when_then_expr = when_then
                    .into_iter()
                    .map(|(when, then)| {
                        let when_expr = Expr::try_from(when).unwrap();
                        let then_expr = Expr::try_from(then).unwrap();

                        (when_expr, then_expr)
                    })
                    .collect();
                let else_result_expr =
                    else_result.map(|expr| Expr::try_from(*expr).map(Box::new).unwrap());

                Ok(Expr::Case {
                    operand: operand_expr,
                    when_then: when_then_expr,
                    else_result: else_result_expr,
                })
            }
        }
    }
}

impl From<&str> for ExprNode {
    fn from(expr: &str) -> Self {
        ExprNode::SqlExpr(expr.to_owned())
    }
}

impl From<i64> for ExprNode {
    fn from(n: i64) -> Self {
        ExprNode::Expr(Expr::Literal(AstLiteral::Number(BigDecimal::from(n))))
    }
}

impl From<Expr> for ExprNode {
    fn from(expr: Expr) -> Self {
        ExprNode::Expr(expr)
    }
}

pub fn expr(value: &str) -> ExprNode {
    ExprNode::from(value)
}

pub fn col(value: &str) -> ExprNode {
    let idents = value.split('.').collect::<Vec<_>>();

    if idents.len() == 1 {
        ExprNode::Identifier(value.to_owned())
    } else {
        ExprNode::CompoundIdentifier(idents.into_iter().map(ToOwned::to_owned).collect())
    }
}

pub fn num(value: i64) -> ExprNode {
    ExprNode::from(value)
}

pub fn text(value: &str) -> ExprNode {
    ExprNode::Expr(Expr::Literal(AstLiteral::QuotedString(value.to_owned())))
}
