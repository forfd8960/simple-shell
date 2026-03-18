use shell_words;

use crate::errors::ShellErrors;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    AndIf,     // &&
    OrIf,      // ||
    Pipe,      // |
    Semicolon, // ;
    Ampersand, // &
    Less,      // <
    Greater,   // >
    DGreater,  // >>
}

pub fn lex_words(cmd_line: &str) -> Vec<String> {
    let lexwords = shell_words::split(cmd_line).unwrap_or_else(|e| {
        eprintln!("Error parsing command line: {}", e);
        Vec::new()
    });
    lexwords
}

pub fn parse_words(words: Vec<String>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for word in words {
        match word.as_str() {
            "&&" => tokens.push(Token::AndIf),
            "||" => tokens.push(Token::OrIf),
            "|" => tokens.push(Token::Pipe),
            ";" => tokens.push(Token::Semicolon),
            "&" => tokens.push(Token::Ampersand),
            "<" => tokens.push(Token::Less),
            ">" => tokens.push(Token::Greater),
            ">>" => tokens.push(Token::DGreater),
            _ => tokens.push(Token::Word(word)),
        }
    }
    tokens
}

/// `Command` 枚举代表了 Shell 语法树中的任意一种命令结构 [2]
#[derive(Debug, Clone)]
pub enum Command {
    /// 顺序执行或后台执行的列表，由 `;` 或 `&` 连接 [3, 9]
    List {
        left: Box<Command>,
        separator: ListSeparator,
        right: Box<Command>,
    },

    /// AND-OR 列表，由 `&&` 或 `||` 连接的命令 [3]
    AndOr {
        left: Box<Command>,
        operator: LogicalOp,
        right: Box<Command>,
    },

    /// 管道命令，由多个子命令通过 `|` 连接，例如 `cmd1 | cmd2` [6, 8]
    Pipeline(Vec<Command>),

    /// 简单命令，例如 `ls -l > out.txt` [6, 7]
    Simple(SimpleCommand),
}

// ---------------------------------------------------------
// 简单命令的具体定义
// ---------------------------------------------------------

/// `SimpleCommand` 包含一个基础命令的所有元素 [7]
#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub cmds: Vec<String>,        // 命令及其参数列表，例如 ["ls", "-l"] [7]
    pub io_rds: Vec<Redirection>, // 该命令的所有 I/O 重定向操作 [16, 17]
}

/// 逻辑操作符 [3, 4]
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOp {
    And, // &&
    Or,  // ||
}

/// 列表分隔符 [3, 15]
#[derive(Debug, Clone, PartialEq)]
pub enum ListSeparator {
    Sequential, // ; (前台顺序执行)
    Async,      // & (放入后台子 Shell 异步执行) [9]
}

// ---------------------------------------------------------
// 重定向操作的具体定义
// ---------------------------------------------------------

/// 表示 I/O 重定向操作，如 `2> error.log` 或 `< input.txt` [16, 17]
#[derive(Debug, Clone)]
pub struct Redirection {
    /// 可选的源文件描述符 (例如 `2>...` 中的 2) [17]
    /// 如果未指定，默认输入为 0，输出为 1 [18, 19]
    pub fd: Option<i32>,

    /// 重定向操作符类型
    pub operator: RedirectOp,

    /// 目标文件名或目标文件描述符 (例如 `&1`) [17-19]
    pub target: String,
}

/// 支持的重定向操作符类型 [17, 20]
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectOp {
    Input,  // <  [21]
    Output, // >  [21]
    Append, // >> [22]
}

pub struct Parser {
    pub tokens: Vec<Token>,
    pub current_pos: usize,
}

/*
list: and_or ((';' | '&') and_or)*
and_or: pipeline (('&&' | '||') pipeline)*
pipeline: command ('|' command)*
command: simple_command | subshell
simple_command: cmd_elements io_redirections*
cmd_elements: WORD cmd_elements?
io_redirections: io_redirect io_redirections?
io_redirect: [n] ('<' | '>' | '>>') WORD
*/

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current_pos: 0,
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current_pos)
    }

    pub fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.current_pos);
        if token.is_some() {
            self.current_pos += 1;
        }
        token
    }

    pub fn consume(&mut self) -> Result<Token, ShellErrors> {
        match self.peek() {
            Some(_) => {
                let tk = self.next().unwrap();
                Ok(tk.clone())
            }
            None => Err(ShellErrors::UnExpectedEndOfInput),
        }
    }

    fn is_end(&self) -> bool {
        self.current_pos > self.tokens.len()
    }

    pub fn parse_tokens(&mut self) -> Result<Vec<Command>, ShellErrors> {
        let mut cmds: Vec<Command> = Vec::new();
        while !self.is_end() {
            cmds.push(self.parse_list()?);
        }

        Ok(cmds)
    }

    fn parse_list(&mut self) -> Result<Command, ShellErrors> {
        let mut left = self.parse_and_or()?;

        while let Some(token) = self.peek() {
            let separator = match token {
                Token::Semicolon => ListSeparator::Sequential,
                Token::Ampersand => ListSeparator::Async,
                _ => break,
            };
            self.next(); // consume the separator

            let right = self.parse_and_or()?;
            left = Command::List {
                left: Box::new(left),
                separator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and_or(&mut self) -> Result<Command, ShellErrors> {
        let mut left = self.parse_pipeline()?;

        while let Some(token) = self.peek() {
            let operator = match token {
                Token::AndIf => LogicalOp::And,
                Token::OrIf => LogicalOp::Or,
                _ => break,
            };
            self.next(); // consume the operator

            let right = self.parse_pipeline()?;
            left = Command::AndOr {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_pipeline(&mut self) -> Result<Command, ShellErrors> {
        let mut simple_commands = Vec::new();
        simple_commands.push(self.parse_simple_command()?);

        loop {
            if let Some(tk) = self.peek()
                && *tk == Token::Pipe
            {
                self.consume()?;
                simple_commands.push(self.parse_simple_command()?);
            } else {
                break;
            }
        }

        Ok(Command::Pipeline(simple_commands))
    }

    fn parse_simple_command(&mut self) -> Result<Command, ShellErrors> {
        let mut words: Vec<String> = Vec::new();
        let mut redirections = Vec::new();
        while let Some(tk) = self.peek() {
            match tk {
                Token::Word(wd) => {
                    words.push(wd.clone());
                    let _ = self.consume()?;
                }
                Token::Less | Token::Greater | Token::DGreater => {
                    let op = self.consume()?;
                    let r_op = token_to_redirect_op(op)?;

                    if let Ok(Token::Word(wd)) = self.consume() {
                        redirections.push(Redirection {
                            fd: None,
                            operator: r_op,
                            target: wd,
                        });
                    } else {
                        return Err(ShellErrors::ExpectedFileName);
                    }
                }
                _ => {}
            }
        }

        Ok(Command::Simple(SimpleCommand {
            cmds: words,
            io_rds: redirections,
        }))
    }
}

pub fn parse_command(cmd_line: &str) -> Vec<Command> {
    let words = lex_words(cmd_line);
    let tokens = parse_words(words);
    println!("Tokens: {:?}", tokens);

    let parser = Parser::new(tokens);

    Vec::new()
}

fn token_to_redirect_op(token: Token) -> Result<RedirectOp, ShellErrors> {
    match token {
        Token::Less => Ok(RedirectOp::Input),
        Token::Greater => Ok(RedirectOp::Output),
        Token::DGreater => Ok(RedirectOp::Append),
        _ => Err(ShellErrors::UnExpectedRedirectOp),
    }
}

#[cfg(test)]
mod tests {
    use super::lex_words;

    #[test]
    fn test_lex() {
        let input = r#"echo "Hello, $USER!""#;
        let words = lex_words(input);
        println!("{:?}", words);

        assert_eq!(words.len(), 2);
        assert_eq!(words[0], "echo");
        assert_eq!(words[1], "Hello, $USER!");
    }

    #[test]
    fn test_lex1() {
        let input = r#"LOG=true cargo run ."#;
        let words = lex_words(input);
        println!("{:?}", words);

        assert_eq!(words.len(), 4);
        assert_eq!(words[0], "LOG=true");
        assert_eq!(words[1], "cargo");
        assert_eq!(words[2], "run");
        assert_eq!(words[3], ".");
    }

    #[test]
    fn test_lex2() {
        let input = r#"cat test.txt | wc -l"#;
        let words = lex_words(input);
        println!("{:?}", words);

        assert_eq!(words.len(), 5);
        assert_eq!(words[0], "cat");
        assert_eq!(words[1], "test.txt");
        assert_eq!(words[2], "|");
        assert_eq!(words[3], "wc");
        assert_eq!(words[4], "-l");
    }
}
