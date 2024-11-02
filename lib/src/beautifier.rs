/*
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
 * the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::max;

use super::args::Arguments;
use anyhow::{anyhow, Context, Result};
use tree_sitter::Node;

struct State<'a> {
    formatted: String,
    arguments: &'a mut Arguments,
    code: &'a [u8],
    col: usize,
    row: usize,
    level: usize,
    extra_indentation: usize,
    num_spaces: usize,
}

impl State<'_> {
    fn indent(&mut self) {
        for _ in 0..self.level {
            self.print(" ".repeat(self.num_spaces).as_str());
        }
        for _ in 0..self.extra_indentation {
            self.print(" ");
        }
    }

    fn print(&mut self, string: &str) {
        if self.arguments.inplace {
            self.formatted += string;
        } else {
            print!("{}", string);
        }
        self.col += string.len();
    }

    fn print_node(&mut self, node: Node) -> Result<()> {
        self.print(node.utf8_text(self.code)?);
        Ok(())
    }

    fn println(&mut self, string: &str) {
        if self.arguments.inplace {
            self.formatted += string;
            self.formatted += "\n";
        } else {
            println!("{}", string);
        }
        self.col = 0;
        self.row += 1;
    }
}

trait TraversingError<T> {
    fn err_at_loc(self, node: &Node) -> Result<T>;
}

impl<T> TraversingError<T> for Option<T> {
    fn err_at_loc(self, node: &Node) -> Result<T> {
        self.ok_or_else(|| {
            anyhow!(
                "Error accessing token around line {} col {}",
                node.range().start_point.row,
                node.range().start_point.column
            )
        })
    }
}

pub fn beautify(code: &str, arguments: &mut Arguments) -> Result<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_ledger::LANGUAGE.into())
        .with_context(|| "Could not set Tree-Sitter language")?;

    let tree = parser
        .parse(code, None)
        .ok_or_else(|| anyhow!("Could not parse file."))?;

    let root = tree.root_node();
    if root.has_error() {
        let error_node = find_first_error_node(root)
            .ok_or_else(|| anyhow!("An error occurred, but no ERROR node was found."))?;
        let line = error_node.start_position().row + 1;
        return Err(anyhow!("Parsed file contain errors (at line {line})."));
    }

    let mut state = State {
        arguments,
        code: code.as_bytes(),
        col: 0,
        row: 0,
        level: 0,
        extra_indentation: 0,
        formatted: String::with_capacity(code.len() * 2),
        num_spaces: 2,
    };

    format_document(&mut state, root)?;
    state.println("");
    Ok(state.formatted)
}

fn find_first_error_node(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if node.is_error() {
        return Some(node);
    }
    for child in node.children(&mut node.walk()) {
        if let Some(error_node) = find_first_error_node(child) {
            return Some(error_node);
        }
    }
    None
}

fn format_document(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    let mut added_newline = false;
    for child in children {
        if child.kind() == "\n" {
            if !added_newline {
                state.println("");
            }
            added_newline = true;
        } else {
            added_newline = false;
            format_journal_item(state, child.child(0).err_at_loc(&node)?)?;
        }
    }
    Ok(())
}

fn format_journal_item(state: &mut State, node: Node) -> Result<()> {
    match node.kind() {
        "comment" => state.print_node(node),
        "block_comment" => state.print_node(node),
        "block_test" => state.print_node(node),
        "directive" => format_directive(state, node),
        "xact" => format_xact(state, node),
        // _ => state.print_node(node),
        _ => Ok(()),
    }
}

fn format_directive(state: &mut State, node: Node) -> Result<()> {
    let child = node.child(0).err_at_loc(&node)?;
    match child.kind() {
        "option" => state.print_node(child),
        "account_directive" => format_account_directive(state, child),
        "commodity_directive" => format_commodity_directive(state, child),
        "tag_directive" => format_tag_directive(state, child),
        "word_directive" => format_word_directive(state, child),
        "char_directive" => format_word_directive(state, child),
        _ => Ok(()),
    }
}

fn format_account_directive(state: &mut State, node: Node) -> Result<()> {
    state.print("account ");
    let account = node
        .named_child(0)
        .err_at_loc(&node)?
        .utf8_text(state.code)?;
    state.println(account);
    let mut cursor = node.walk();
    let children: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "account_subdirective")
        .collect();
    state.level += 1;
    for child in children {
        let child = child.child(0).err_at_loc(&child)?;
        match child.kind() {
            "alias_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "alias")?
            }
            "note_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "note")?
            }
            "assert_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "assert")?
            }
            "check_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "check")?
            }
            "payee_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "payee")?
            }
            "default_subdirective" => {
                state.indent();
                state.println("default");
            }
            _ => continue,
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_commodity_directive(state: &mut State, node: Node) -> Result<()> {
    state.print("commodity ");
    let commodity = node
        .named_child(0)
        .err_at_loc(&node)?
        .utf8_text(state.code)?;
    state.println(commodity);
    let mut cursor = node.walk();
    let children: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "commodity_subdirective")
        .collect();
    state.level += 1;
    for child in children {
        let child = child.child(0).err_at_loc(&child)?;
        match child.kind() {
            "alias_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "alias")?;
            }
            "note_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "note")?;
            }
            "format_subdirective" => {
                state.indent();
                format_format_subdirective(state, child)?;
            }
            "default_subdirective" => {
                state.indent();
                state.println("default");
            }
            "nomarket_subdirective" => {
                state.indent();
                state.println("nomarket");
            }
            _ => continue,
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_tag_directive(state: &mut State, node: Node) -> Result<()> {
    state.print("tag ");
    let mut cursor = node.walk();
    let tag = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "tag")
        .err_at_loc(&node)?
        .utf8_text(state.code)?
        .trim();
    state.println(tag);
    state.level += 1;
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "assert_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "assert")?
            }
            "check_subdirective" => {
                state.indent();
                format_argument_subdirective(state, child, "check")?
            }
            _ => continue,
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_word_directive(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.children(&mut cursor) {
        if child.kind() == "whitespace" {
            continue;
        }
        let value = child.utf8_text(state.code)?.trim();
        if value.is_empty() {
            continue;
        }
        if !first {
            state.print(" ");
        }
        state.print(value);
        first = false;
    }
    state.println("");
    Ok(())
}

fn format_argument_subdirective(state: &mut State, node: Node, argument: &str) -> Result<()> {
    state.print(argument);
    state.print(" ");
    let mut cursor = node.walk();
    let alias = node
        .children(&mut cursor)
        .find(|c| c.kind() == "value")
        .err_at_loc(&node)?;
    state.print(alias.utf8_text(state.code)?);
    Ok(())
}

fn format_format_subdirective(state: &mut State, node: Node) -> Result<()> {
    state.print("format ");
    let mut cursor = node.walk();
    let amount = node
        .children(&mut cursor)
        .find(|c| c.kind() == "amount")
        .err_at_loc(&node)?;
    format_amount(state, amount)?;
    Ok(())
}

fn format_xact(state: &mut State, node: Node) -> Result<()> {
    let child = node.child(0).err_at_loc(&node)?;
    match child.kind() {
        "plain_xact" => format_plain_xact(state, child)?,
        "periodic_xact" => format_periodic_xact(state, child)?,
        "automated_xact" => format_automated_xact(state, child)?,
        _ => {}
    }
    Ok(())
}

fn format_plain_xact(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let types_first_line = ["date", "effective_date", "status", "code", "payee"];
    for child in node
        .named_children(&mut cursor)
        .filter(|c| types_first_line.contains(&c.kind()))
    {
        let value = child.utf8_text(state.code)?;
        match child.kind() {
            "date" => {
                state.print(value);
            }
            "effective_date" => {
                state.print("=");
                state.print(value);
            }
            "status" | "code" | "payee" => {
                state.print(" ");
                state.print(value);
            }
            _ => {}
        }
    }
    state.println("");
    state.level += 1;
    for child in node
        .named_children(&mut cursor)
        .filter(|c| !types_first_line.contains(&c.kind()))
    {
        let value = child.utf8_text(state.code)?;
        match child.kind() {
            "note" => {
                state.indent();
                state.println(value);
            }
            "posting" => {
                state.indent();
                format_posting(state, child)?;
            }
            _ => {}
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_periodic_xact(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    state.print("~ ");
    let interval = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "interval")
        .err_at_loc(&node)?
        .utf8_text(state.code)?
        .trim();
    state.print(interval);
    if let Some(note) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "note")
    {
        state.print(" ");
        state.print(note.utf8_text(state.code)?);
    }
    state.println("");
    state.level += 1;
    let types_first_line = ["note", "interval"];
    for child in node
        .named_children(&mut cursor)
        .filter(|c| !types_first_line.contains(&c.kind()))
    {
        let value = child.utf8_text(state.code)?;
        match child.kind() {
            "note" => {
                state.indent();
                state.println(value);
            }
            "posting" => {
                state.indent();
                format_posting(state, child)?;
            }
            _ => {}
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_automated_xact(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    state.print("= ");
    let query = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "query")
        .err_at_loc(&node)?
        .utf8_text(state.code)?
        .trim();
    state.print(query);
    if let Some(note) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "note")
    {
        state.print(" ");
        state.print(note.utf8_text(state.code)?);
    }
    state.println("");
    state.level += 1;
    let types_first_line = ["note", "query"];
    for child in node
        .named_children(&mut cursor)
        .filter(|c| !types_first_line.contains(&c.kind()))
    {
        let value = child.utf8_text(state.code)?;
        match child.kind() {
            "note" => {
                state.indent();
                state.println(value);
            }
            "posting" => {
                state.indent();
                format_posting(state, child)?;
            }
            _ => {}
        }
    }
    state.level -= 1;
    Ok(())
}

fn format_posting(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    if let Some(status) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "status")
    {
        let text = status.utf8_text(state.code)?;
        state.print(text);
    }
    if let Some(account) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "account")
    {
        let text = account.utf8_text(state.code)?;
        state.print(text);
    }
    let mut spacing = " ".repeat(max(0, 60 - state.col));
    if let Some(amount) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "amount")
    {
        let mut cursor = amount.walk();
        let number_size = amount
            .named_children(&mut cursor)
            .find(|c| c.kind() == "quantity" || c.kind() == "negative_quantity")
            .err_at_loc(&amount)?
            .utf8_text(state.code)?
            .trim()
            .len();
        let quantity_spacing = " ".repeat(max(0, 60 - state.col - number_size - 1));
        state.print(&quantity_spacing);
        format_amount(state, amount)?;
        spacing = " ".into();
    }
    if let Some(price) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "price")
    {
        state.print(&spacing);
        format_price(state, price)?;
        spacing = " ".into();
    }
    if let Some(balance_assertion) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "balance_assertion")
    {
        state.print(&spacing);
        format_balance_assertion(state, balance_assertion)?;
        spacing = " ".into();
    }
    if let Some(note) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "note")
    {
        state.print(&spacing);
        state.print(note.utf8_text(state.code)?.trim());
    }
    state.println("");
    Ok(())
}

fn format_amount(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    if let Some(negative_quantity) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "negative_quantity")
    {
        state.print(negative_quantity.utf8_text(state.code)?.trim());
    }
    if let Some(quantity) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "quantity")
    {
        state.print(quantity.utf8_text(state.code)?.trim());
    }
    if let Some(commodity) = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "commodity")
    {
        state.print(" ");
        state.print(commodity.utf8_text(state.code)?.trim());
    }
    Ok(())
}

fn format_price(state: &mut State, node: Node) -> Result<()> {
    state.print(node.child(0).err_at_loc(&node)?.utf8_text(state.code)?);
    state.print(" ");
    let mut cursor = node.walk();
    let amount = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "amount")
        .err_at_loc(&node)?;
    format_amount(state, amount)
}

fn format_balance_assertion(state: &mut State, node: Node) -> Result<()> {
    state.print("= ");
    let mut cursor = node.walk();
    let amount = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "amount")
        .err_at_loc(&node)?;
    format_amount(state, amount)
}
