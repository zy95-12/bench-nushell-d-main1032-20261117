//! Validate lexical loop control without allowing a loop to cross a closure boundary.
use nu_protocol::{
    CompileError, DeclId,
    ast::{Block, Expr, Expression, PipelineRedirection, RecordItem, RedirectionTarget},
    engine::StateWorkingSet,
};

pub(crate) fn validate(working_set: &StateWorkingSet, block: &Block) -> Vec<CompileError> {
    let mut errors = Vec::new();
    visit_block(working_set, block, false, &mut errors);
    // Imported module definitions need validation even when the import call's
    // expression does not contain their bodies.
    for index in working_set.permanent_state.num_decls()..working_set.num_decls() {
        if let Some(id) = working_set.get_decl(DeclId::new(index)).block_id() {
            visit_block(working_set, working_set.get_block(id), false, &mut errors);
        }
    }
    let mut seen = std::collections::HashSet::new();
    errors.retain(|error| match error {
        CompileError::NotInALoop { span, .. } => {
            seen.insert(span.map(|span| (span.start, span.end)))
        }
        _ => true,
    });
    errors
}

fn visit_block(ws: &StateWorkingSet, block: &Block, in_loop: bool, errors: &mut Vec<CompileError>) {
    for pipeline in &block.pipelines {
        for element in &pipeline.elements {
            visit(ws, &element.expr, in_loop, errors);
            let mut target = |target: &RedirectionTarget| {
                if let RedirectionTarget::File { expr, .. } = target {
                    visit(ws, expr, in_loop, errors);
                }
            };
            match &element.redirection {
                Some(PipelineRedirection::Single { target: t, .. }) => target(t),
                Some(PipelineRedirection::Separate { out, err }) => {
                    target(out);
                    target(err);
                }
                None => {}
            }
        }
    }
}

fn visit(ws: &StateWorkingSet, expr: &Expression, in_loop: bool, errors: &mut Vec<CompileError>) {
    let mut child = |expr: &Expression| visit(ws, expr, in_loop, errors);
    match &expr.expr {
        Expr::Call(call) => {
            let decl = ws.get_decl(call.decl_id);
            let name = decl.name();
            if decl.is_keyword() && matches!(name, "break" | "continue") && !in_loop {
                errors.push(CompileError::NotInALoop {
                    msg: format!("{name} must be used inside a loop"),
                    span: Some(call.head),
                });
            }
            for arg in &call.arguments {
                if let Some(expr) = arg.expr() {
                    // A literal catch body is inlined by the compiler. Although
                    // represented as a closure, it shares the surrounding loop.
                    if decl.is_keyword() && name == "try" {
                        if let Expr::Keyword(keyword) = &expr.expr {
                            if keyword.keyword.as_ref() == b"catch" {
                                if let Some(id) = keyword.expr.as_block() {
                                    visit_block(ws, ws.get_block(id), in_loop, errors);
                                    continue;
                                }
                            }
                        }
                    }
                    let context = if decl.is_keyword() && matches!(name, "def" | "export def") {
                        false
                    } else if decl.is_keyword()
                        && (name == "while"
                            || (matches!(name, "for" | "loop")
                                && matches!(expr.expr, Expr::Block(_))))
                    {
                        true
                    } else {
                        in_loop
                    };
                    visit(ws, expr, context, errors);
                }
            }
        }
        Expr::Block(id) | Expr::Subexpression(id) => {
            visit_block(ws, ws.get_block(*id), in_loop, errors)
        }
        Expr::Closure(id) | Expr::RowCondition(id) => {
            visit_block(ws, ws.get_block(*id), false, errors)
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            child(lhs);
            child(op);
            child(rhs);
        }
        Expr::UnaryNot(expr) | Expr::Collect(_, expr) => child(expr),
        Expr::FullCellPath(path) => child(&path.head),
        Expr::Keyword(keyword) => child(&keyword.expr),
        Expr::AttributeBlock(block) => child(&block.item),
        Expr::StringInterpolation(parts) | Expr::GlobInterpolation(parts, _) => {
            for part in parts {
                child(part);
            }
        }
        Expr::List(items) => {
            for item in items {
                child(item.expr());
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair(key, value) => {
                        child(key);
                        child(value);
                    }
                    RecordItem::Spread(_, expr) => child(expr),
                }
            }
        }
        Expr::Table(table) => {
            for header in &table.columns {
                child(header);
            }
            for row in &table.rows {
                for cell in row {
                    child(cell);
                }
            }
        }
        Expr::MatchBlock(arms) => {
            for (pattern, expr) in arms {
                if let Some(guard) = &pattern.guard {
                    child(guard);
                }
                child(expr);
            }
        }
        Expr::Range(range) => {
            for expr in [&range.from, &range.next, &range.to].into_iter().flatten() {
                child(expr);
            }
        }
        Expr::ExternalCall(head, args) => {
            child(head);
            for arg in args {
                child(arg.expr());
            }
        }
        _ => {}
    }
}
