pub mod cmd;
pub mod errors;
pub(crate) mod expand;
pub mod ioloop;
pub(crate) mod parser;
pub(crate) mod state;

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

/// `Command` 枚举代表了 Shell 语法树中的任意一种命令结构 [2]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 顺序执行或后台执行的列表，由 `;` 或 `&` 连接 [3, 9]
    List {
        left: Box<Command>,
        separator: ListSeparator,
        right: Option<Box<Command>>,
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

impl Command {
    /// 尝试将 `Command` 转换为 `SimpleCommand`，如果不是简单命令则返回 `None`
    pub fn as_simple(&self) -> Option<&SimpleCommand> {
        if let Command::Simple(simple) = self {
            Some(simple)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------
// 简单命令的具体定义
// ---------------------------------------------------------

/// `SimpleCommand` 包含一个基础命令的所有元素 [7]
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCommand {
    pub cmds: Vec<String>,        // 命令及其参数列表，例如 ["ls", "-l"] [7]
    pub io_rds: Vec<Redirection>, // 该命令的所有 I/O 重定向操作 [16, 17]
}

/// 逻辑操作符 [3, 4]
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOp {
    /// Run if previous succeeded
    And, // &&
    /// Run if previous failed
    Or, // ||
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
#[derive(Debug, Clone, PartialEq)]
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
