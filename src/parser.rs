use shell_words;

use crate::{
    Command, ListSeparator, LogicalOp, RedirectOp, Redirection, SimpleCommand, Token,
    errors::ShellErrors,
};

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
        self.current_pos >= self.tokens.len()
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

            let _ = self.consume()?; // consume the separator

            // if cmd line is end after ; or &
            if self.is_end() {
                left = Command::List {
                    left: Box::new(left),
                    separator,
                    right: None,
                };
                break;
            } else {
                let right = self.parse_and_or()?;
                left = Command::List {
                    left: Box::new(left),
                    separator,
                    right: Some(Box::new(right)),
                };
            }
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

        if simple_commands.len() == 1 {
            return Ok(simple_commands[0].clone());
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
                _ => {
                    break;
                }
            }
        }

        Ok(Command::Simple(SimpleCommand {
            cmds: words,
            io_rds: redirections,
        }))
    }
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
    use std::vec;

    use crate::{errors::ShellErrors, parser::{
        Command, ListSeparator, LogicalOp, Parser, RedirectOp, Redirection, SimpleCommand, parse_words
    }};

    use super::lex_words;

    fn parse_command(cmd_line: &str) -> Result<Vec<Command>, ShellErrors> {
        let words = lex_words(cmd_line);
        let tokens = parse_words(words);
        println!("Tokens: {:?}", tokens);

        let mut parser = Parser::new(tokens);

        println!("parsing into AST");
        parser.parse_tokens()
    }

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

    #[test]
    fn test_parse1() {
        let input = r#"echo "Hello, $USER!""#;
        let ast_result = parse_command(input);
        println!("{:?}", ast_result);

        assert!(ast_result.is_ok());

        let ast = ast_result.unwrap();
        println!("ast: {:?}", ast);
        assert_eq!(
            ast,
            vec![Command::Simple(SimpleCommand {
                cmds: vec!["echo".to_string(), "Hello, $USER!".to_string()],
                io_rds: vec![],
            })]
        );
    }

    #[test]
    fn test_01_single_word_command() {
        // 测试目的：验证最基本的单个词能否被解析为 SimpleCommand [2]
        let ast = parse_command("ls").unwrap();

        assert_eq!(ast.len(), 1);

        assert_eq!(
            ast[0],
            Command::Simple(SimpleCommand {
                cmds: vec!["ls".to_string()],
                io_rds: vec![]
            })
        );
    }

    #[test]
    fn test_parser() {
        let inputs = vec![r#"ls -l > out.txt &"#];

        let expect_asts = vec![vec![Command::List {
            left: Box::new(Command::Simple(SimpleCommand {
                cmds: vec!["ls".to_string(), "-l".to_string()],
                io_rds: vec![Redirection {
                    fd: None,
                    operator: RedirectOp::Output,
                    target: "out.txt".to_string(),
                }],
            })),
            separator: ListSeparator::Async,
            right: None,
        }]];

        for (idx, input) in inputs.iter().enumerate() {
            let ast_result = parse_command(input);
            println!("{:?}", ast_result);

            assert!(ast_result.is_ok());

            let ast = ast_result.unwrap();
            println!("ast: {:?}", ast);
            assert_eq!(ast, expect_asts[idx]);
        }
    }

    #[test]
    fn test_03_input_redirection() {
        // 测试目的：验证输入重定向 '<' 是否能正确提取目标文件名 [4, 5]
        let ast = parse_command("cat < input.txt").unwrap();
        match &ast[0] {
            Command::Simple(cmd) => {
                assert_eq!(cmd.cmds, vec!["cat"]);
                assert_eq!(cmd.io_rds.len(), 1);
                assert_eq!(cmd.io_rds[0].operator, RedirectOp::Input);
                assert_eq!(cmd.io_rds[0].target, "input.txt");
            }
            _ => panic!("Expected Simple command"),
        }
    }

    #[test]
    fn test_04_multiple_redirections() {
        // 测试目的：POSIX 规定一个命令可包含多个重定向操作符，按从左到右顺序求值 [4, 6]
        let ast = parse_command("grep error < file.log > out.txt").unwrap();
        match &ast[0] {
            Command::Simple(cmd) => {
                assert_eq!(cmd.cmds, vec!["grep", "error"]);
                assert_eq!(cmd.io_rds.len(), 2);
                assert_eq!(cmd.io_rds[0].operator, RedirectOp::Input);
                assert_eq!(cmd.io_rds[0].target, "file.log");
                assert_eq!(cmd.io_rds[1].operator, RedirectOp::Output);
                assert_eq!(cmd.io_rds[1].target, "out.txt");
            }
            _ => panic!("Expected Simple command"),
        }
    }

    #[test]
    fn test_06_multi_stage_pipeline() {
        // 测试目的：验证长管道能否被展平为长度大于2的 Vec 数组，避免深度嵌套 [8]
        let ast = parse_command("cat file.txt | grep error | sort | uniq").unwrap();
        match &ast[0] {
            Command::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 4); // 应该包含 4 个连续的简单命令
                assert_eq!(
                    cmds[0],
                    Command::Simple(SimpleCommand {
                        cmds: vec!["cat".to_string(), "file.txt".to_string()],
                        io_rds: vec![]
                    })
                );
                assert_eq!(
                    cmds[1],
                    Command::Simple(SimpleCommand {
                        cmds: vec!["grep".to_string(), "error".to_string()],
                        io_rds: vec![]
                    })
                );
                assert_eq!(
                    cmds[2],
                    Command::Simple(SimpleCommand {
                        cmds: vec!["sort".to_string()],
                        io_rds: vec![]
                    })
                );
                assert_eq!(
                    cmds[3],
                    Command::Simple(SimpleCommand {
                        cmds: vec!["uniq".to_string()],
                        io_rds: vec![]
                    })
                );
            }
            _ => panic!("Expected Pipeline command"),
        }
    }

    #[test]
    fn test_07_logical_and() {
        // 测试目的：验证 "&&" 能否正确构建具有左结合性的 AndOr 树 [10, 11]
        let ast = parse_command("make clean && make all").unwrap();
        match &ast[0] {
            Command::AndOr {
                left,
                operator,
                right,
            } => {
                assert_eq!(*operator, LogicalOp::And);
                assert_eq!(
                    *left,
                    Box::new(Command::Simple(SimpleCommand {
                        cmds: vec!["make".to_string(), "clean".to_string()],
                        io_rds: vec![]
                    }))
                );
                assert_eq!(
                    *right,
                    Box::new(Command::Simple(SimpleCommand {
                        cmds: vec!["make".to_string(), "all".to_string()],
                        io_rds: vec![]
                    }))
                );
            }
            _ => panic!("Expected AndOr command"),
        }
    }

    #[test]
    fn test_08_logical_or_with_pipeline() {
        // 测试目的：验证 "||" 分隔的元素可以是一个 Pipeline（Pipeline 的优先级高于 AndOr） [10, 12]
        let ast = parse_command("cat config.json | grep port || echo 'port not found'").unwrap();

        let cmd = &ast[0];
        println!("{:?}", cmd);

        assert_eq!(
            *cmd,
            Command::AndOr {
                left: Box::new(Command::Pipeline(vec![
                    Command::Simple(SimpleCommand {
                        cmds: vec!["cat".to_string(), "config.json".to_string()],
                        io_rds: vec![]
                    }),
                    Command::Simple(SimpleCommand {
                        cmds: vec!["grep".to_string(), "port".to_string()],
                        io_rds: vec![]
                    })
                ])),
                operator: LogicalOp::Or,
                right: Box::new(Command::Simple(SimpleCommand {
                    cmds: vec!["echo".to_string(), "port not found".to_string()],
                    io_rds: vec![]
                }))
            }
        );
    }

    #[test]
    fn test_09_sequential_list() {
        // 测试目的：验证由 ';' 分隔的命令被解析为顺序执行的 List [10, 13]
        let ast = parse_command("cd /var/log ; ls -l").unwrap();
        assert_eq!(
            ast,
            vec![Command::List {
                left: Box::new(Command::Simple(SimpleCommand {
                    cmds: vec!["cd".to_string(), "/var/log".to_string()],
                    io_rds: vec![],
                })),
                separator: ListSeparator::Sequential,
                right: Some(Box::new(Command::Simple(SimpleCommand {
                    cmds: vec!["ls".to_string(), "-l".to_string()],
                    io_rds: vec![],
                })))
            }]
        )
    }

    #[test]
    fn test_10_asynchronous_list() {
        // 测试目的：验证带有 '&' 后缀的命令被解析为异步（后台）作业 [14]
        // 按照规范，"&" 也是一种分隔符或终止符
        let ast = parse_command("sleep 10 & echo 'done'").unwrap();
        assert_eq!(
            ast,
            vec![Command::List {
                left: Box::new(Command::Simple(SimpleCommand {
                    cmds: vec!["sleep".to_string(), "10".to_string()],
                    io_rds: vec![],
                })),
                separator: ListSeparator::Async,
                right: Some(Box::new(Command::Simple(SimpleCommand {
                    cmds: vec!["echo".to_string(), "done".to_string()],
                    io_rds: vec![],
                })))
            }]
        )
    }
}
