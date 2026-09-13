// The terminal interface. It owns the screen and the keys; every change to
// the list goes through the record of functions morloc hands it, so the
// interface never opens the file and never sees how a task is stored.
//
// Two things make a full-screen program awkward inside a morloc pool, and
// both are handled here. The pool is put in its own process group by the
// nexus, so it is not the terminal's foreground group and a raw-mode read
// would take SIGTTIN and stop; and the pool's stdout belongs to the nexus,
// which prints this function's result to it when it returns. So the
// interface is drawn on /dev/tty directly, the foreground group is claimed
// on entry, and a guard restores both the terminal mode and the original
// group on the way out.

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::terminal;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

#[derive(Clone, Debug)]
pub struct Todo {
    pub title: String,
    pub priority: i64,
    pub done: bool,
    pub tags: Vec<String>,
}

// The morloc core. Each field is a callback into the pool that owns the
// list; a field of no arguments is a suspension the interface runs once
// per use. Do not `use rustmorloc::MorlocFnN` here: the generated pool
// already imports the traits, so they are named in full.
#[derive(Clone)]
pub struct Api {
    pub list: std::rc::Rc<dyn rustmorloc::MorlocFn0<Vec<Todo>>>,
    pub add: std::rc::Rc<dyn rustmorloc::MorlocFn3<String, i64, Vec<String>, Vec<Todo>>>,
    pub check: std::rc::Rc<dyn rustmorloc::MorlocFn1<i64, Vec<Todo>>>,
    pub drop: std::rc::Rc<dyn rustmorloc::MorlocFn1<i64, Vec<Todo>>>,
    pub clean: std::rc::Rc<dyn rustmorloc::MorlocFn0<Vec<Todo>>>,
    pub sort: std::rc::Rc<dyn rustmorloc::MorlocFn0<Vec<Todo>>>,
}

// Restores the terminal however the interface leaves: clean exit, error,
// or panic.
struct TtyGuard {
    fd: i32,
    pgrp: i32,
}

impl Drop for TtyGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        if self.pgrp > 0 {
            unsafe {
                libc::tcsetpgrp(self.fd, self.pgrp);
            }
        }
    }
}

fn claim_tty(fd: i32) -> i32 {
    unsafe {
        // Taking the foreground group is itself a background write, so the
        // stop signals have to be ignored before asking for it.
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
        let prev = libc::tcgetpgrp(fd);
        libc::tcsetpgrp(fd, libc::getpgrp());
        prev
    }
}

// What the interface is doing: browsing the list, or typing a new task.
enum Mode {
    Browse,
    Add { text: String, priority: i64 },
}

struct State {
    todos: Vec<Todo>,
    sel: usize,
    mode: Mode,
    status: String,
}

impl State {
    fn set(&mut self, todos: Vec<Todo>) {
        self.todos = todos;
        if self.sel >= self.todos.len() {
            self.sel = self.todos.len().saturating_sub(1);
        }
    }
}

// Words that start with '#' are tags, the rest is the title.
fn split_tags(text: &str) -> (String, Vec<String>) {
    let mut title = Vec::new();
    let mut tags = Vec::new();
    for w in text.split_whitespace() {
        match w.strip_prefix('#') {
            Some(t) if !t.is_empty() => tags.push(t.to_string()),
            _ => title.push(w),
        }
    }
    (title.join(" "), tags)
}

fn bar(priority: i64) -> &'static str {
    match priority {
        p if p >= 3 => "!!!",
        2 => "!! ",
        _ => "!  ",
    }
}

fn draw(f: &mut ratatui::Frame, st: &State) {
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .areas(f.area());

    let left = st.todos.iter().filter(|t| !t.done).count();
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" todo ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} tasks, {} left", st.todos.len(), left)),
        ])),
        head,
    );

    let items: Vec<ListItem> = if st.todos.is_empty() {
        vec![ListItem::new(Span::styled(
            "(nothing to do)",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        st.todos
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut spans = vec![
                    Span::raw(format!("{:>2} ", i)),
                    Span::raw(if t.done { "[x] " } else { "[ ] " }),
                    Span::styled(
                        format!("{} ", bar(t.priority)),
                        Style::default().fg(match t.priority {
                            p if p >= 3 => Color::Red,
                            2 => Color::Yellow,
                            _ => Color::DarkGray,
                        }),
                    ),
                    Span::styled(
                        t.title.clone(),
                        if t.done {
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
                        } else {
                            Style::default()
                        },
                    ),
                ];
                for tag in &t.tags {
                    spans.push(Span::styled(format!("  #{}", tag), Style::default().fg(Color::Cyan)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    let mut ls = ListState::default();
    if !st.todos.is_empty() {
        ls.select(Some(st.sel));
    }
    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        body,
        &mut ls,
    );

    let lines = match &st.mode {
        Mode::Browse => vec![
            Line::from(" j/k move  space toggle  a add  d delete  s sort  c clean  r reload  q quit"),
            Line::from(Span::styled(format!(" {}", st.status), Style::default().fg(Color::DarkGray))),
        ],
        Mode::Add { text, priority } => vec![
            Line::from(vec![
                Span::raw(format!(" new [{}] ", bar(*priority).trim_end())),
                Span::raw(text.clone()),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ]),
            Line::from(Span::styled(
                " enter add  tab priority  esc cancel  (#word adds a tag)",
                Style::default().fg(Color::DarkGray),
            )),
        ],
    };
    f.render_widget(Paragraph::new(lines), foot);
}

pub fn todo_tui(api: &Api) -> String {
    let tty = match std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty") {
        Ok(f) => f,
        Err(_) => rustmorloc::morloc_throw(
            "todo tui needs a terminal: /dev/tty could not be opened".to_string(),
        ),
    };
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&tty);
    let prev = claim_tty(fd);
    let _guard = TtyGuard { fd, pgrp: prev };

    if terminal::enable_raw_mode().is_err() {
        rustmorloc::morloc_throw("could not put the terminal into raw mode".to_string());
    }
    let mut out = tty;
    let _ = ratatui::crossterm::execute!(out, terminal::EnterAlternateScreen);
    let mut term = match ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(out)) {
        Ok(t) => t,
        Err(e) => rustmorloc::morloc_throw(format!("terminal: {}", e)),
    };

    let mut st = State {
        todos: api.list.call0(),
        sel: 0,
        mode: Mode::Browse,
        status: String::new(),
    };

    loop {
        if term.draw(|f| draw(f, &st)).is_err() {
            break;
        }
        let key = match event::read() {
            Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => k,
            Ok(_) => continue,
            Err(_) => break,
        };
        // Taken out by value: an arm that edits the list must not hold a
        // borrow of the mode while it does.
        match std::mem::replace(&mut st.mode, Mode::Browse) {
            Mode::Add { mut text, priority } => match key.code {
                KeyCode::Esc => {}
                KeyCode::Tab => st.mode = Mode::Add { text, priority: priority % 3 + 1 },
                KeyCode::Backspace => {
                    text.pop();
                    st.mode = Mode::Add { text, priority };
                }
                KeyCode::Char(c) => {
                    text.push(c);
                    st.mode = Mode::Add { text, priority };
                }
                KeyCode::Enter => {
                    let (title, tags) = split_tags(&text);
                    if !title.is_empty() {
                        let n = st.todos.len();
                        let todos = api.add.call3(&title, &priority, &tags);
                        st.set(todos);
                        st.sel = n;
                        st.status = format!("added {}", title);
                    }
                }
                _ => st.mode = Mode::Add { text, priority },
            },
            Mode::Browse => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('j') | KeyCode::Down => {
                    if st.sel + 1 < st.todos.len() {
                        st.sel += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => st.sel = st.sel.saturating_sub(1),
                KeyCode::Char('g') => st.sel = 0,
                KeyCode::Char('G') => st.sel = st.todos.len().saturating_sub(1),
                KeyCode::Char(' ') | KeyCode::Char('x') | KeyCode::Enter => {
                    if !st.todos.is_empty() {
                        let todos = api.check.call1(&(st.sel as i64));
                        st.set(todos);
                    }
                }
                KeyCode::Char('d') => {
                    if !st.todos.is_empty() {
                        st.status = format!("dropped {}", st.todos[st.sel].title);
                        let todos = api.drop.call1(&(st.sel as i64));
                        st.set(todos);
                    }
                }
                KeyCode::Char('a') => st.mode = Mode::Add { text: String::new(), priority: 1 },
                KeyCode::Char('s') => {
                    let todos = api.sort.call0();
                    st.set(todos);
                    st.status = "sorted by priority".to_string();
                }
                KeyCode::Char('c') => {
                    let todos = api.clean.call0();
                    st.set(todos);
                    st.status = "dropped completed tasks".to_string();
                }
                KeyCode::Char('r') => {
                    let todos = api.list.call0();
                    st.set(todos);
                    st.status = "reloaded".to_string();
                }
                _ => {}
            },
        }
    }

    let _ = ratatui::crossterm::execute!(term.backend_mut(), terminal::LeaveAlternateScreen);
    let _ = term.show_cursor();
    let left = st.todos.iter().filter(|t| !t.done).count();
    format!("{} tasks, {} left", st.todos.len(), left)
}
