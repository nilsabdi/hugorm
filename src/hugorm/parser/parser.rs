use super::super::error::Response::Wrong;
use super::*;

use std::rc::Rc;

pub struct Parser<'p> {
    index: usize,
    tokens: Vec<Token>,
    source: &'p Source,

    indent_standard: usize,
    indent: usize,

    min_prec: usize,
}

impl<'p> Parser<'p> {
    pub fn new(tokens: Vec<Token>, source: &'p Source) -> Self {
        Parser {
            tokens,
            source,
            index: 0,

            indent_standard: 0,
            indent: 0,

            min_prec: 0
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, ()> {
        let mut ast = Vec::new();

        while self.remaining() > 0 {
            ast.push(self.parse_statement()?)
        }

        Ok(ast)
    }

    pub fn parse_statement(&mut self) -> Result<Statement, ()> {
        use self::TokenType::*;

        while self.current_type() == EOL && self.remaining() != 0 {
            self.next()?
        }

        let position = self.current_position();

        let statement = match self.current_type() {
            Keyword => match self.current_lexeme().as_str() {
                "return" => {
                    self.next()?;

                    if self.current_lexeme() == "\n" {
                        Statement::new(
                            StatementNode::Return(
                                None
                            ),
                            position
                        )
                    } else {
                        Statement::new(
                            StatementNode::Return(
                                Some(self.parse_expression()?)
                            ),
                            self.span_from(position)
                        )
                    }
                }

                "let" => {
                    self.next()?;

                    let name = self.eat_type(&TokenType::Identifier)?;

                    if self.current_lexeme() == "\n" {
                        Statement::new(
                            StatementNode::Declaration(
                                name,
                                None
                            ),
                            self.span_from(position)
                        )
                    } else {
                        self.eat_lexeme("=")?;

                        let right = self.parse_expression()?;

                        Statement::new(
                            StatementNode::Declaration(
                                name,
                                Some(right)
                            ),
                            self.span_from(position)
                        )
                    }
                }

                "const" => {
                    self.next()?;

                    if self.current_lexeme() == "fun" {
                        Statement::new(
                            StatementNode::ConstFunction(
                                Rc::new(self.parse_statement()?)
                            ),
                            self.span_from(position)
                        )
                    } else {
                        let name = self.eat_type(&TokenType::Identifier)?;

                        self.eat_lexeme("=")?;

                        let right = self.parse_expression()?;

                        Statement::new(
                            StatementNode::Declaration(
                                name,
                                Some(right)
                            ),
                            self.span_from(position)
                        )
                    }
                }

                "fun" => {
                    self.next()?;

                    let name = self.eat_type(&TokenType::Identifier)?;
                    
                    let new_pos = self.span_from(position);

                    self.eat_lexeme("(")?;
                    self.next_newline()?;

                    let mut params = Vec::new();

                    if self.current_lexeme() != ")" {
                        params.push(self.eat_type(&TokenType::Identifier)?);

                        while self.current_lexeme() == "," {
                            self.next()?;
                            self.next_newline()?;
                            
                            params.push(self.eat_type(&TokenType::Identifier)?)
                        }
                    }

                    self.eat_lexeme(")")?;
                    self.eat_lexeme(":")?;

                    let body = if self.current_lexeme() == "\n" {
                        self.next()?;
                        self.parse_body()?
                    } else {
                        vec!(self.parse_statement()?)
                    };

                    return Ok(
                        Statement::new(
                            StatementNode::Function(
                                name,
                                params,
                                body
                            ),
                            new_pos
                        )
                    )
                },

                "interface" => {
                    self.next()?;

                    let name = self.eat_type(&TokenType::Identifier)?;
                    let new_pos = self.span_from(position);

                    self.eat_lexeme(":")?;

                    let body = if self.current_lexeme() == "\n" {
                        self.next()?;
                        self.parse_body()?
                    } else {
                        vec!(self.parse_statement()?)
                    };

                    for s in body.iter() {
                        if let StatementNode::Function(..) = s.node {
                            continue
                        } else {
                            return Err(response!(
                                Wrong(format!("can't interface non-function")),
                                self.source.file,
                                s.pos
                            ));
                        }
                    }

                    return Ok(
                        Statement::new(
                            StatementNode::Interface(
                                name,
                                body
                            ),
                            new_pos
                        )
                    )
                },

                "while" => {
                    self.next()?;

                    let cond = self.parse_expression()?;

                    self.eat_lexeme(":")?;

                    let pos = self.span_from(position);

                    let body = if self.current_lexeme() == "\n" {
                        self.next()?;
                        self.parse_body()?
                    } else {
                        vec!(self.parse_statement()?)
                    };

                    return Ok(
                        Statement::new(
                            StatementNode::While(cond, body),
                            pos
                        )
                    )
                }

                "loop" => {
                    self.next()?;

                    if self.current_lexeme() == ":" {
                        self.next()?;

                        let pos = self.span_from(position);

                        let cond = Expression::new(
                            ExpressionNode::Bool(true),
                            pos.clone()
                        );

                        let body = if self.current_lexeme() == "\n" {
                            self.next()?;
                            self.parse_body()?
                        } else {
                            vec!(self.parse_statement()?)
                        };

                        return Ok(
                            Statement::new(
                                StatementNode::While(cond, body),
                                pos
                            )
                        )
                    } else {
                        let count = self.parse_expression()?;

                        self.eat_lexeme(":")?;

                        let pos = self.span_from(position);

                        let name = format!("$loopy-boi-{}", self.remaining()); // we can do this, the programmer can't

                        let iterator = Statement::new(
                            StatementNode::Declaration(
                                name.clone(),
                                Some(
                                    Expression::new(
                                    ExpressionNode::Int(0),
                                    pos.clone()
                                    )
                                ),
                            ),
                            pos.clone()
                        );

                        let left = Expression::new(
                            ExpressionNode::Identifier(name),
                            pos.clone()
                        );

                        let increment = Statement::new(
                            StatementNode::Assignment(
                                left.clone(),
                                Expression::new(
                                    ExpressionNode::Binary(
                                        Rc::new(left.clone()),
                                        super::Operator::Add,
                                        Rc::new(
                                            Expression::new(
                                                ExpressionNode::Int(1),
                                                pos.clone()
                                            )
                                        ),
                                    ),
                                    pos.clone()
                                )
                            ),
                            pos.clone()
                        ); 

                        let comp = Expression::new(
                            ExpressionNode::Binary(
                                Rc::new(left.clone()),
                                super::Operator::Lt,
                                Rc::new(
                                    count
                                ),
                            ),
                            pos.clone()
                        );

                        let mut body = if self.current_lexeme() == "\n" {
                            self.next()?;
                            self.parse_body()?
                        } else {
                            vec!(self.parse_statement()?)
                        };

                        body.push(increment);

                        let loopy = Statement::new(
                            StatementNode::Block(
                                vec![
                                    iterator,
                                    Statement::new(
                                        StatementNode::While(
                                            comp,
                                            body
                                        ),
                                        pos.clone()
                                    )
                                ]
                            ),
                            pos
                        );

                        return Ok(
                            loopy
                        )
                    }
                }

                "break" => {
                    self.next()?;

                    Statement::new(
                        StatementNode::Break,
                        position
                    )
                }

                

                "if" => {
                    self.next()?;

                    let cond = self.parse_expression()?;

                    self.eat_lexeme(":")?;

                    let pos = self.span_from(position);

                    let mut no_else = false;

                    let body = if self.current_lexeme() == "\n" {
                        self.next()?;
                        self.parse_body()?
                    } else {
                        no_else = true;
                        vec!(self.parse_statement()?)
                    };
                    
                    if no_else {
                        return Ok(
                            Statement::new(
                                StatementNode::If(cond, body, Vec::new()),
                                pos
                            )
                        )
                    } else {
                        let mut else_ = Vec::new();

                        let mut cur = self.current_lexeme();

                        while ["elif", "else"].contains(&cur.as_str()) {
                            self.next()?;
                            
                            if cur == "else" {
                                self.eat_lexeme(":")?;

                                let body = if self.current_lexeme() == "\n" {
                                    self.next()?;
                                    self.parse_body()?
                                } else {
                                    vec!(self.parse_statement()?)
                                };

                                else_.push((None, body))
                            } else if cur == "elif" {
                                let cond = self.parse_expression()?;
                                self.eat_lexeme(":")?;

                                let body = if self.current_lexeme() == "\n" {
                                    self.next()?;
                                    self.parse_body()?
                                } else {
                                    vec!(self.parse_statement()?)
                                };

                                else_.push((Some(cond), body))
                            }

                            cur = self.current_lexeme()
                        }

                        return Ok(
                            Statement::new(
                                StatementNode::If(cond, body, else_),
                                pos
                            )
                        )
                    }
                }

                "unless" => {
                    self.next()?;

                    let mut cond = self.parse_expression()?;

                    cond.node = ExpressionNode::Not(Rc::new(cond.clone()));

                    self.eat_lexeme(":")?;

                    let pos = self.span_from(position);

                    let mut no_else = false;

                    let body = if self.current_lexeme() == "\n" {
                        self.next()?;
                        self.parse_body()?
                    } else {
                        no_else = true;
                        vec!(self.parse_statement()?)
                    };
                    
                    if no_else {
                        return Ok(
                            Statement::new(
                                StatementNode::If(cond, body, Vec::new()),
                                pos
                            )
                        )
                    } else {
                        let mut else_ = Vec::new();

                        let mut cur = self.current_lexeme();

                        while ["elif", "else"].contains(&cur.as_str()) {
                            self.next()?;
                            
                            if cur == "else" {
                                self.eat_lexeme(":")?;

                                let body = if self.current_lexeme() == "\n" {
                                    self.next()?;
                                    self.parse_body()?
                                } else {
                                    vec!(self.parse_statement()?)
                                };

                                else_.push((None, body))
                            } else if cur == "elif" {
                                let cond = self.parse_expression()?;
                                self.eat_lexeme(":")?;

                                let body = if self.current_lexeme() == "\n" {
                                    self.next()?;
                                    self.parse_body()?
                                } else {
                                    vec!(self.parse_statement()?)
                                };

                                else_.push((Some(cond), body))
                            }

                            cur = self.current_lexeme()
                        }

                        return Ok(
                            Statement::new(
                                StatementNode::If(cond, body, else_),
                                pos
                            )
                        )
                    }
                }

                _ => {
                    let expression = self.parse_expression()?;
                    let position = expression.pos.clone();
    
                    Statement::new(StatementNode::Expression(expression), position)
                }
            },

            _ => {
                let expression = self.parse_expression()?;
                let position = expression.pos.clone();

                if let Some(result) = self.try_parse_compound(&expression)? {
                    result
                } else {
                    if self.current_lexeme() == "=" {
                        self.next()?;

                        Statement::new(
                            StatementNode::Assignment(expression, self.parse_expression()?),
                            position,
                        )
                    } else {
                        Statement::new(StatementNode::Expression(expression), position)
                    }
                }
            }
        };

        self.new_line()?;

        Ok(statement)
    }

    fn try_parse_compound(&mut self, left: &Expression) -> Result<Option<Statement>, ()> {
        if self.current_type() != TokenType::Operator {
            return Ok(None)
        }

        let backup_index = self.index;

        let c = self.eat_type(&TokenType::Operator)?;

        let mut result = None;

        if self::Operator::is_compoundable(&c) {
            let op = self::Operator::from_str(&c).unwrap().0;

            let position = self.current_position();

            if self.current_lexeme() == "=" {
                self.next()?;

                let right = self.parse_expression()?;
                let ass = Statement::new(
                    StatementNode::Assignment(
                        left.clone(),
                        Expression::new(
                            ExpressionNode::Binary(Rc::new(left.clone()), op, Rc::new(right)),
                            self.span_from(position.clone()),
                        ),
                    ),
                    self.span_from(position),
                );

                result = Some(ass)
            } else {
                self.index = backup_index
            }
        }

        Ok(result)
    }

    fn parse_body(&mut self) -> Result<Vec<Statement>, ()> {
        let backup_indent = self.indent;
        self.indent = self.get_indent();

        if self.indent_standard == 0 {
            self.indent_standard = self.indent
        } else {
            if self.indent % self.indent_standard != 0 {
                return Err(response!(
                    Wrong(format!("found inconsistently indented token")),
                    self.source.file,
                    self.current_position()
                ));
            }
        }

        let mut stack = Vec::new();

        while !self.is_dedent() && self.remaining() > 0 {
            let statement = self.parse_statement()?;

            self.next_newline()?;

            stack.push(statement)
        }

        self.indent = backup_indent;

        Ok(stack)
    }

    fn parse_expression(&mut self) -> Result<Expression, ()> {
        let atom = self.parse_atom()?;

        if self.current_type() == TokenType::Operator {
            self.parse_binary(atom, 0)
        } else {
            Ok(atom)
        }
    }

    fn parse_atom(&mut self) -> Result<Expression, ()> {
        use self::TokenType::*;

        if self.remaining() == 0 {
            Ok(Expression::new(
                ExpressionNode::EOF,
                self.current_position(),
            ))
        } else {
            let token_type = self.current_type().clone();
            let position = self.current_position();

            let expression = match token_type {
                Int => Expression::new(
                    ExpressionNode::Int(self.eat()?.parse::<i32>().unwrap()),
                    position,
                ),

                Float => Expression::new(
                    ExpressionNode::Float(self.eat()?.parse::<f64>().unwrap()),
                    position,
                ),

                Str => Expression::new(ExpressionNode::Str(self.eat()?), position),

                Bool => Expression::new(ExpressionNode::Bool(self.eat()? == "true"), position),

                Identifier => Expression::new(ExpressionNode::Identifier(self.eat()?), position),

                Operator => match self.current_lexeme().as_str() {
                    "-" => {
                        self.next()?;

                        Expression::new(
                            ExpressionNode::Neg(Rc::new(self.parse_expression()?)),
                            self.span_from(position),
                        )
                    }

                    "not" => {
                        self.next()?;

                        Expression::new(
                            ExpressionNode::Not(Rc::new(self.parse_expression()?)),
                            self.span_from(position),
                        )
                    }

                    ref op => {
                        return Err(response!(
                            Wrong(format!("unexpected operator `{}`", op)),
                            self.source.file,
                            self.current_position()
                        ))
                    }
                },

                Symbol => match self.current_lexeme().as_str() {
                    "(" => {
                        self.next()?;
                        self.next_newline()?;

                        if self.current_lexeme() == ")" && self.current_type() == TokenType::Symbol
                        {
                            self.next()?;

                            Expression::new(ExpressionNode::Empty, self.span_from(position))
                        } else {
                            let expression = self.parse_expression()?;

                            self.eat_lexeme(")")?;

                            expression
                        }
                    }

                    "[" => {
                        let expr = Expression::new(
                            ExpressionNode::Array(
                                self.parse_block_of(("[", "]"), &Self::_parse_expression_comma)?,
                            ),
                            self.span_from(position),
                        );

                        expr
                    },

                    "{" => {
                        let args =
                                self.parse_block_of(("{", "}"), &Self::_parse_definition_comma)?;

                        Expression::new(
                            ExpressionNode::Dict(args),
                            self.span_from(position)
                        )
                    },

                    ref c => {
                        return Err(response!(
                            Wrong(format!("unexpected symbol `{}`", c)),
                            self.source.file,
                            self.current_position()
                        ))
                    }
                },

                Keyword => match self.current_lexeme().as_str() {
                    "nil" => Expression::new(
                        ExpressionNode::Nil,
                        position
                    ),

                    "fun" => {
                        self.next()?;
                        
                        let name = format!("<anon-fn ${}>", self.remaining());

                        let new_pos = self.span_from(position);
    
                        self.eat_lexeme("(")?;
                        self.next_newline()?;
    
                        let mut params = Vec::new();
    
                        if self.current_lexeme() != ")" {
                            params.push(self.eat_type(&TokenType::Identifier)?);
    
                            while self.current_lexeme() == "," {
                                self.next()?;
                                self.next_newline()?;
                                
                                params.push(self.eat_type(&TokenType::Identifier)?)
                            }
                        }
    
                        self.eat_lexeme(")")?;
                        self.eat_lexeme(":")?;
    
                        let body = if self.current_lexeme() == "\n" {
                            self.next()?;
                            self.parse_body()?
                        } else {
                            vec!(self.parse_statement()?)
                        };
    
                        return Ok(
                            Expression::new(
                                ExpressionNode::AnonFunction(
                                    name,
                                    params,
                                    body
                                ),
                                new_pos
                            )
                        )
                    },

                    c => return Err(response!(
                        Wrong(format!("unexpected keyword `{}`", c)),
                        self.source.file,
                        self.current_position()
                    ))
                },

                ref token_type => {
                    return Err(response!(
                        Wrong(format!("unexpected token `{}`", token_type)),
                        self.source.file,
                        self.current_position()
                    ))
                }
            };

            if self.remaining() > 0 {
                self.parse_postfix(expression)
            } else {
                Ok(expression)
            }
        }
    }

    fn parse_postfix(&mut self, expression: Expression) -> Result<Expression, ()> {
        let backup_index = self.index;

        if self.remaining() == 0 {
            return Ok(expression);
        }

        match self.current_type() {
            TokenType::Symbol => match self.current_lexeme().as_str() {
                "(" => {
                    self.next()?;
                    self.next_newline()?;

                    let mut args = Vec::new();

                    if ![TokenType::Operator, TokenType::Keyword].contains(&self.current_type())
                    {
                        while !["\n", ")"].contains(&self.current_lexeme().as_str()) {
                            args.push(self.parse_expression()?);

                            if !["\n", ")"].contains(&self.current_lexeme().as_str())
                                && self.remaining() > 0
                            {
                                self.eat_lexeme(",")?;
                                self.next_newline()?;
                            }
                        }
                    }

                    self.next_newline()?;
                    self.eat_lexeme(")")?;

                    let position = expression.pos.clone();

                    let call = Expression::new(
                        ExpressionNode::Call(Rc::new(expression), args),
                        self.span_from(position),
                    );

                    self.parse_postfix(call)
                },

                "[" => {
                    self.next()?;

                    let expr = self.parse_expression()?;

                    self.eat_lexeme("]")?;

                    let position = expression.pos.clone();

                    let index = Expression::new(
                        ExpressionNode::Binary(Rc::new(expression), Operator::Index, Rc::new(expr)),
                        self.span_from(position),
                    );

                    self.parse_postfix(index)
                }

                "." => {
                    self.next()?;

                    let position = self.current_position();

                    let id = Expression::new(ExpressionNode::Str(self.eat()?), position);

                    let position = expression.pos.clone();

                    let index = Expression::new(
                        ExpressionNode::Binary(Rc::new(expression), Operator::Index, Rc::new(id)),
                        self.span_from(position),
                    );

                    self.parse_postfix(index)
                }
                
                _ => {
                    Ok(expression)
                }
            },

            TokenType::Keyword => match self.current_lexeme().as_str() {
                "with" => {
                    self.next()?;

                    let with = self.parse_expression()?;

                    let pos = expression.pos.clone();

                    return Ok(Expression::new(
                        ExpressionNode::With(Rc::new(expression), Rc::new(with)),
                        self.span_from(pos)
                    ))
                }

                _ => {
                    self.index = backup_index;
                    Ok(expression)
                }
            }

            _ => Ok(expression)
        }
    }

    fn parse_binary(&mut self, left: Expression, min_prec: usize) -> Result<Expression, ()> {
        let mut left = left;
        let left_position = left.pos.clone();

        while self.current_type() == TokenType::Operator {
            let index_backup = self.index;
            let operator = Operator::from_str(self.eat()?.as_str()).unwrap();

            if operator.1 < min_prec as u8 {
                println!("we've reached a bruh moment: {:#?} @ {} {}", operator.0, operator.1, min_prec);
                self.index = index_backup;
                break
            }

            let prec = if !operator.0.is_right_ass() {
                operator.1 + 1
            } else {
                operator.1
            };

            let mut right = self.parse_atom()?;
            right = self.parse_binary(right, prec as usize)?;

            left = Expression::new(
                ExpressionNode::Binary(
                    Rc::new(left),
                    operator.0,
                    Rc::new(right.clone())
                ),
                self.span_from(left_position.clone())
            );
        }

        println!("next: {}", self.current_lexeme());

        Ok(left)
    }

    fn new_line(&mut self) -> Result<(), ()> {
        if self.remaining() > 0 {
            match self.current_lexeme().as_str() {
                "\n" => self.next(),
                _ => {
                    Err(response!(
                        Wrong(format!(
                            "expected new line found: `{}`",
                            self.current_lexeme()
                        )),
                        self.source.file,
                        self.current_position()
                    ))
                },
            }
        } else {
            Ok(())
        }
    }

    fn next_newline(&mut self) -> Result<(), ()> {
        while self.current_lexeme() == "\n" && self.remaining() > 0 {
            self.next()?
        }

        Ok(())
    }

    fn get_indent(&self) -> usize {
        self.current().slice.0 - 1
    }

    fn is_dedent(&self) -> bool {
        self.get_indent() < self.indent && self.current_lexeme() != "\n"
    }

    fn next(&mut self) -> Result<(), ()> {
        if self.index <= self.tokens.len() {
            self.index += 1;

            Ok(())
        } else {
            Err(response!(
                Wrong("moving outside token stack"),
                self.source.file,
                self.current_position()
            ))
        }
    }

    fn remaining(&self) -> usize {
        self.tokens.len().saturating_sub(self.index)
    }

    fn current_position(&self) -> Pos {
        let current = self.current();

        Pos(current.line.clone(), current.slice)
    }

    fn span_from(&self, left_position: Pos) -> Pos {
        let Pos(ref line, ref slice) = left_position;
        let Pos(_, ref slice2) = self.current_position();

        Pos(
            line.clone(),
            (
                slice.0,
                if slice2.1 < line.1.len() {
                    slice2.1
                } else {
                    line.1.len()
                },
            ),
        )
    }

    fn current(&self) -> Token {
        if self.index > self.tokens.len() - 1 {
            self.tokens[self.tokens.len() - 1].clone()
        } else {
            self.tokens[self.index].clone()
        }
    }

    fn eat(&mut self) -> Result<String, ()> {
        let lexeme = self.current().lexeme;
        self.next()?;

        Ok(lexeme)
    }

    fn eat_lexeme(&mut self, lexeme: &str) -> Result<String, ()> {
        if self.current_lexeme() == lexeme {
            let lexeme = self.current().lexeme;
            self.next()?;

            Ok(lexeme)
        } else {
            Err(response!(
                Wrong(format!(
                    "expected `{}` but found `{}`",
                    lexeme,
                    self.current_lexeme()
                )),
                self.source.file,
                self.current_position()
            ))
        }
    }

    fn eat_type(&mut self, token_type: &TokenType) -> Result<String, ()> {
        if self.current_type() == *token_type {
            let lexeme = self.current().lexeme.clone();
            self.next()?;

            Ok(lexeme)
        } else {
            Err(response!(
                Wrong(format!(
                    "expected `{}` but found `{}`",
                    token_type,
                    self.current_type()
                )),
                self.source.file,
                self.current_position()
            ))
        }
    }

    fn current_lexeme(&self) -> String {
        self.current().lexeme.clone()
    }

    fn current_type(&self) -> TokenType {
        self.current().token_type
    }

    fn expect_type(&self, token_type: TokenType) -> Result<(), ()> {
        if self.current_type() == token_type {
            Ok(())
        } else {
            Err(response!(
                Wrong(format!(
                    "expected `{}` but found `{}`",
                    token_type,
                    self.current_type()
                )),
                self.source.file
            ))
        }
    }

    fn expect_lexeme(&self, lexeme: &str) -> Result<(), ()> {
        if self.current_lexeme() == lexeme {
            Ok(())
        } else {
            Err(response!(
                Wrong(format!(
                    "expected `{}` but found `{}`",
                    lexeme,
                    self.current_lexeme()
                )),
                self.source.file
            ))
        }
    }



    fn _parse_statement(self: &mut Self) -> Result<Option<Statement>, ()> {
        if self.remaining() > 0 {
            Ok(Some(self.parse_statement()?))
        } else {
            Ok(None)
        }
    }

    fn _parse_expression(self: &mut Self) -> Result<Option<Expression>, ()> {
        let expression = self.parse_expression()?;

        match expression.node {
            ExpressionNode::EOF => Ok(None),
            _ => Ok(Some(expression)),
        }
    }

    fn _parse_expression_comma(self: &mut Self) -> Result<Option<Expression>, ()> {
        if self.remaining() > 0 && self.current_lexeme() == "\n" {
            self.next()?
        }

        let expression = Self::_parse_expression(self);

        if self.remaining() > 0 && self.current_lexeme() == "\n" {
            self.next()?
        }

        if self.remaining() > 0 {
            self.eat_lexeme(",")?;

            if self.remaining() > 0 && self.current_lexeme() == "\n" {
                self.next()?
            }
        }

        expression
    }

    fn _parse_definition_comma(self: &mut Self) -> Result<Option<(String, Expression)>, ()> {
        if self.remaining() > 0 && self.current_lexeme() == "\n" {
            self.next()?
        }

        if self.remaining() == 0 {
            return Ok(None);
        }

        let position = self.current_position();

        let name = self.eat_type(&TokenType::Identifier)?;

        self.eat_lexeme(":")?;

        let mut value = self.parse_expression()?;

        value.pos = position;

        let param = Some((name, value));

        if self.remaining() > 0 {
            if ![",", "\n"].contains(&self.current_lexeme().as_str()) {
                return Err(response!(
                    Wrong(format!(
                        "expected `,` or newline, found `{}`",
                        self.current_lexeme()
                    )),
                    self.source.file,
                    self.current_position()
                ));
            } else {
                self.next()?;
            }

            if self.remaining() > 0 && self.current_lexeme() == "\n" {
                self.next()?
            }
        }

        Ok(param)
    }



    fn parse_block_of<B>(
        &mut self,
        delimeters: (&str, &str),
        parse_with: &dyn Fn(&mut Self) -> Result<Option<B>, ()>,
    ) -> Result<Vec<B>, ()> {
        self.eat_lexeme(delimeters.0)?;

        if self.current_lexeme() == delimeters.1 {
            self.next()?;

            return Ok(Vec::new());
        }

        let mut block_tokens = Vec::new();
        let mut nest_count = 1;

        while nest_count > 0 {
            if self.current_lexeme() == delimeters.1 && self.current_type() == TokenType::Symbol {
                nest_count -= 1
            } else if self.current_lexeme() == delimeters.0
                && self.current_type() == TokenType::Symbol
            {
                nest_count += 1
            }

            if nest_count == 0 {
                break;
            } else {
                block_tokens.push(self.current());

                self.next()?;
            }
        }

        self.eat_lexeme(delimeters.1)?;

        if !block_tokens.is_empty() {
            let mut parser = Parser::new(block_tokens, self.source);
            let mut block = Vec::new();

            while let Some(element) = parse_with(&mut parser)? {
                block.push(element)
            }

            Ok(block)
        } else {
            Ok(Vec::new())
        }
    }
}