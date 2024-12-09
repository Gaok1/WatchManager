use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::text::Span;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, List, ListItem, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
enum Modo {
    Cadastro,
    Buscar,
    Historico,
    Estoques,
    Grafico,
    Compra,
    Venda,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Relogio {
    codigo: String,
    quantidade: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Historico {
    codigo: String,
    quantidade: i32,
    operacao: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
struct Persist {
    relogios: Vec<Relogio>,
    historico: Vec<Historico>,
}

enum HistoricoTab {
    Todos,
    Compras,
    Vendas,
    Aquisicoes,
}

impl HistoricoTab {
    fn titles() -> &'static [&'static str] {
        &["Todos", "Compras", "Vendas", "Aquisições"]
    }
    fn next(&self) -> HistoricoTab {
        match self {
            HistoricoTab::Todos => HistoricoTab::Compras,
            HistoricoTab::Compras => HistoricoTab::Vendas,
            HistoricoTab::Vendas => HistoricoTab::Aquisicoes,
            HistoricoTab::Aquisicoes => HistoricoTab::Todos,
        }
    }
    fn prev(&self) -> HistoricoTab {
        match self {
            HistoricoTab::Todos => HistoricoTab::Aquisicoes,
            HistoricoTab::Compras => HistoricoTab::Todos,
            HistoricoTab::Vendas => HistoricoTab::Compras,
            HistoricoTab::Aquisicoes => HistoricoTab::Vendas,
        }
    }
}

struct App {
    relogios: HashMap<String, Relogio>,
    historico: Vec<Historico>,
    modo: Modo,
    input: String,
    mensagens: Vec<String>,
    historico_filtrado: Option<Vec<Historico>>,
    editing: bool,
    estoques_list: Vec<Relogio>,
    estoques_offset: usize,
    estoques_selected: usize,
    historico_offset: usize,
    historico_selected: usize,
    historico_tab: HistoricoTab,

    cadastro_list: Vec<Relogio>,
    cadastro_offset: usize,
    cadastro_selected: usize,

    buscar_results: Vec<(String, i32, usize)>,
    buscar_offset: usize,
    buscar_selected: usize,

    chosen_relogio: Option<String>,
    chosen_operation: Option<char>,

    // Novos campos para pesquisa no histórico
    historico_codigos_unicos: Vec<String>,
    historico_search_results: Vec<(String, usize)>,
    historico_search_selected: usize, // índice na lista de sugestões
}

impl App {
    fn new() -> Self {
        let (relogios, hist) = load_from_file();

        // Extrair códigos únicos do histórico
        let mut cod_set: HashSet<String> = HashSet::new();
        for h in &hist {
            cod_set.insert(h.codigo.clone());
        }
        let mut historico_codigos_unicos: Vec<String> = cod_set.into_iter().collect();
        historico_codigos_unicos.sort();

        let mut app = Self {
            relogios,
            historico: hist,
            modo: Modo::Estoques,
            input: String::new(),
            mensagens: vec!["Bem-vindo ao Sistema de Relógios (Estoque)!".into()],
            historico_filtrado: None,
            editing: false,
            estoques_list: vec![],
            estoques_offset: 0,
            estoques_selected: 0,
            historico_offset: 0,
            historico_selected: 0,
            historico_tab: HistoricoTab::Todos,
            cadastro_list: vec![],
            cadastro_offset: 0,
            cadastro_selected: 0,
            buscar_results: vec![],
            buscar_offset: 0,
            buscar_selected: 0,
            chosen_relogio: None,
            chosen_operation: None,
            historico_codigos_unicos,
            historico_search_results: vec![],
            historico_search_selected: 0,
        };
        app.atualiza_estoques_list();
        app.atualiza_cadastro_list();
        app
    }

    fn atualiza_estoques_list(&mut self) {
        let mut lista: Vec<Relogio> = self.relogios.values().cloned().collect();
        lista.sort_by(|a, b| a.codigo.cmp(&b.codigo));
        self.estoques_list = lista;
        if self.estoques_offset >= self.estoques_list.len() && !self.estoques_list.is_empty() {
            self.estoques_offset = self.estoques_list.len() - 1;
        }
        if self.estoques_selected >= self.estoques_list.len() && !self.estoques_list.is_empty() {
            self.estoques_selected = self.estoques_list.len() - 1;
        }
    }

    fn atualiza_cadastro_list(&mut self) {
        let mut lista: Vec<Relogio> = self.relogios.values().cloned().collect();
        lista.sort_by(|a, b| a.codigo.cmp(&b.codigo));
        self.cadastro_list = lista;
        if self.cadastro_offset >= self.cadastro_list.len() && !self.cadastro_list.is_empty() {
            self.cadastro_offset = self.cadastro_list.len() - 1;
        }
        if self.cadastro_selected >= self.cadastro_list.len() && !self.cadastro_list.is_empty() {
            self.cadastro_selected = self.cadastro_list.len() - 1;
        }
    }

    fn atualizar_busca_results(&mut self) {
        let results = self.busca_relogios(&self.input);
        self.buscar_results = results;
        if self.buscar_offset >= self.buscar_results.len() && !self.buscar_results.is_empty() {
            self.buscar_offset = self.buscar_results.len() - 1;
        }
        if self.buscar_selected >= self.buscar_results.len() && !self.buscar_results.is_empty() {
            self.buscar_selected = self.buscar_results.len() - 1;
        }
    }

    fn atualizar_historico_search_results(&mut self) {
        // Calcula a distância para cada código com base em self.input
        let query = self.input.trim();
        let mut res: Vec<(String, usize)> = self
            .historico_codigos_unicos
            .iter()
            .map(|c| {
                let dist = levenshtein_distance(c, query);
                (c.clone(), dist)
            })
            .collect();
        res.sort_by_key(|(_, d)| *d);
        self.historico_search_results = res;
        if !self.historico_search_results.is_empty() {
            if self.historico_search_selected >= self.historico_search_results.len() {
                self.historico_search_selected = self.historico_search_results.len() - 1;
            }
        } else {
            self.historico_search_selected = 0;
        }
    }

    fn cadastrar_relogio(&mut self, codigo: String, qtd: i32) {
        let r = Relogio {
            codigo: codigo.clone(),
            quantidade: qtd,
        };
        self.relogios.insert(codigo.clone(), r);
        self.historico.push(Historico {
            codigo: codigo.clone(),
            quantidade: qtd,
            operacao: "CADASTRO".into(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        self.mensagens.push(format!(
            "Relógio {} cadastrado com {} unidades",
            codigo, qtd
        ));
        self.atualiza_estoques_list();
        self.atualiza_cadastro_list();
        save_to_file(&self.relogios, &self.historico);
    }

    fn vender_relogio(&mut self, codigo: String, qtd: i32) {
        if let Some(r) = self.relogios.get_mut(&codigo) {
            if r.quantidade >= qtd {
                r.quantidade -= qtd;
                self.historico.push(Historico {
                    codigo: codigo.clone(),
                    quantidade: qtd,
                    operacao: "VENDA".into(),
                    timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                });
                self.mensagens
                    .push(format!("Vendido {} unidades do relógio {}", qtd, codigo));
            } else {
                self.mensagens
                    .push("Não há estoque suficiente para vender!".into());
            }
        } else {
            self.mensagens.push("Relógio não encontrado!".into());
        }
        self.atualiza_estoques_list();
        self.atualiza_cadastro_list();
        save_to_file(&self.relogios, &self.historico);
    }

    fn comprar_relogio(&mut self, codigo: String, qtd: i32) {
        if let Some(r) = self.relogios.get_mut(&codigo) {
            r.quantidade += qtd;
        } else {
            self.relogios.insert(
                codigo.clone(),
                Relogio {
                    codigo: codigo.clone(),
                    quantidade: qtd,
                },
            );
        }
        self.historico.push(Historico {
            codigo: codigo.clone(),
            quantidade: qtd,
            operacao: "COMPRA".into(),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        self.mensagens
            .push(format!("Adicionado {} unidades do relógio {}", qtd, codigo));
        self.atualiza_estoques_list();
        self.atualiza_cadastro_list();
        save_to_file(&self.relogios, &self.historico);
    }

    fn get_historico_atual_vec(&self) -> Vec<Historico> {
        let base = if let Some(ref h) = self.historico_filtrado {
            h
        } else {
            &self.historico
        };
        base.iter()
            .filter(|h| match self.historico_tab {
                HistoricoTab::Todos => true,
                HistoricoTab::Compras => h.operacao == "COMPRA",
                HistoricoTab::Vendas => h.operacao == "VENDA",
                HistoricoTab::Aquisicoes => h.operacao == "CADASTRO",
            })
            .cloned()
            .collect()
    }

    fn get_historico_por_codigo(&self, codigo: &str) -> Vec<Historico> {
        self.historico
            .iter()
            .filter(|h| h.codigo == codigo)
            .cloned()
            .collect()
    }

    fn entra_modo_insercao(&mut self, modo: Modo) {
        self.modo = modo;
        self.input.clear();
        self.editing = true;
        self.historico_filtrado = None;
    }

    fn sai_modo_insercao(&mut self) {
        self.modo = Modo::Estoques;
        self.input.clear();
        self.editing = false;
        self.historico_filtrado = None;
    }

    fn busca_relogios(&self, query: &str) -> Vec<(String, i32, usize)> {
        let mut resultados: Vec<(String, i32, usize)> = self
            .relogios
            .values()
            .map(|r| {
                let dist = levenshtein_distance(&r.codigo, query);
                (r.codigo.clone(), r.quantidade, dist)
            })
            .collect();
        resultados.sort_by_key(|r| r.2);
        resultados
    }

    fn formata_data_ddmm(data: &str) -> String {
        let parts: Vec<&str> = data.split('-').collect();
        if parts.len() == 3 {
            let dia = parts[2];
            let mes = parts[1];
            format!("{}/{}", dia, mes)
        } else {
            data.to_string()
        }
    }

    fn agrupamento_por_dia(&self) -> Vec<(String, usize, usize)> {
        let mut mapa: HashMap<String, (usize, usize)> = HashMap::new();

        for h in &self.historico {
            if let Some(space_pos) = h.timestamp.find(' ') {
                let data_str = &h.timestamp[..space_pos];
                let entry = mapa.entry(data_str.to_string()).or_insert((0, 0));
                match h.operacao.as_str() {
                    "VENDA" => entry.0 += 1,
                    "COMPRA" => entry.1 += 1,
                    _ => {}
                }
            }
        }

        let mut vet: Vec<(String, (usize, usize))> = mapa.into_iter().collect();
        vet.sort_by(|a, b| a.0.cmp(&b.0));
        let len = vet.len();
        let start = if len > 7 { len - 7 } else { 0 };
        vet[start..]
            .iter()
            .map(|(k, (v, c))| {
                let curta = Self::formata_data_ddmm(k);
                (curta, *v, *c)
            })
            .collect()
    }

    fn historico_select_up(&mut self) {
        if self.historico_selected > 0 {
            self.historico_selected -= 1;
            if self.historico_selected < self.historico_offset {
                self.historico_offset = self.historico_selected;
            }
        }
    }

    fn historico_select_down(&mut self) {
        let data = self.get_historico_atual_vec();
        if !data.is_empty() && self.historico_selected + 1 < data.len() {
            self.historico_selected += 1;
            let vis_height = 5;
            if self.historico_selected >= self.historico_offset + vis_height {
                self.historico_offset = self.historico_selected - vis_height + 1;
            }
        }
    }

    fn historico_tab_next(&mut self) {
        self.historico_tab = self.historico_tab.next();
        self.historico_selected = 0;
        self.historico_offset = 0;
    }

    fn historico_tab_prev(&mut self) {
        self.historico_tab = self.historico_tab.prev();
        self.historico_selected = 0;
        self.historico_offset = 0;
    }

    fn cadastro_select_up(&mut self) {
        if self.cadastro_selected > 0 {
            self.cadastro_selected -= 1;
            if self.cadastro_selected < self.cadastro_offset {
                self.cadastro_offset = self.cadastro_selected;
            }
        }
    }

    fn cadastro_select_down(&mut self) {
        let total = self.cadastro_list.len();
        if !self.cadastro_list.is_empty() && self.cadastro_selected + 1 < total {
            self.cadastro_selected += 1;
            let vis_height = 5;
            if self.cadastro_selected >= self.cadastro_offset + vis_height {
                self.cadastro_offset = self.cadastro_selected - vis_height + 1;
            }
        }
    }

    fn buscar_select_up(&mut self) {
        if self.buscar_selected > 0 {
            self.buscar_selected -= 1;
            if self.buscar_selected < self.buscar_offset {
                self.buscar_offset = self.buscar_selected;
            }
        }
    }

    fn buscar_select_down(&mut self) {
        let total = self.buscar_results.len();
        if !self.buscar_results.is_empty() && self.buscar_selected + 1 < total {
            self.buscar_selected += 1;
            let vis_height = 5;
            if self.buscar_selected >= self.buscar_offset + vis_height {
                self.buscar_offset = self.buscar_selected - vis_height + 1;
            }
        }
    }

    fn historico_search_up(&mut self) {
        if self.historico_search_selected > 0 {
            self.historico_search_selected -= 1;
        }
    }

    fn historico_search_down(&mut self) {
        if !self.historico_search_results.is_empty()
            && self.historico_search_selected + 1 < self.historico_search_results.len()
        {
            self.historico_search_selected += 1;
        }
    }

    fn selecionar_registro(&mut self, codigo: String) {
        self.chosen_relogio = Some(codigo.clone());
        self.chosen_operation = None;
        self.mensagens.push(format!(
            "Registro {} selecionado. Aperte A ou V para escolher operação.",
            codigo
        ));
    }

    fn cancelar_selecao(&mut self) {
        self.chosen_relogio = None;
        self.chosen_operation = None;
        self.mensagens.push("Seleção cancelada.".into());
    }

    fn escolher_operacao(&mut self, op: char) {
        if let Some(cod) = &self.chosen_relogio {
            self.chosen_operation = Some(op);
            self.mensagens
                .push(format!("Operação '{}' selecionada para {}", op, cod));
            if op == 'A' {
                self.modo = Modo::Compra;
                self.editing = true;
                self.input = format!("{} ", cod);
            } else if op == 'V' {
                self.modo = Modo::Venda;
                self.editing = true;
                self.input = format!("{} ", cod);
            }
        }
    }

    fn filtrar_historico(&mut self, codigo: &str) {
        if codigo.is_empty() {
            self.historico_filtrado = None;
            self.mensagens
                .push("Filtro removido. Mostrando todo o histórico.".into());
        } else {
            let hist = self.get_historico_por_codigo(codigo);
            if hist.is_empty() {
                self.mensagens
                    .push("Nenhum histórico encontrado para esse código!".into());
                self.historico_filtrado = None;
            } else {
                self.historico_filtrado = Some(hist);
                self.mensagens
                    .push(format!("Histórico filtrado por {} exibido", codigo));
            }
        }
        self.historico_offset = 0;
        self.historico_selected = 0;
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let mut costs = (0..=b.len()).collect::<Vec<_>>();
    for (i, ca) in a.chars().enumerate() {
        costs[0] = i + 1;
        let mut corner = i;
        for (j, cb) in b.chars().enumerate() {
            let upper = costs[j + 1];
            if ca == cb {
                costs[j + 1] = corner;
            } else {
                let t = corner.min(upper).min(costs[j]) + 1;
                costs[j + 1] = t;
            }
            corner = upper;
        }
    }
    costs[b.len()]
}

fn load_from_file() -> (HashMap<String, Relogio>, Vec<Historico>) {
    let mut relogios: HashMap<String, Relogio> = HashMap::new();
    let mut historico: Vec<Historico> = vec![];

    if let Ok(data) = fs::read_to_string("estoque.json") {
        if let Ok(json) = serde_json::from_str::<Persist>(&data) {
            relogios = json
                .relogios
                .into_iter()
                .map(|r| (r.codigo.clone(), r))
                .collect();
            historico = json.historico;
        }
    }
    (relogios, historico)
}

fn save_to_file(relogios: &HashMap<String, Relogio>, historico: &Vec<Historico>) {
    let r: Vec<Relogio> = relogios.values().cloned().collect();
    let p = Persist {
        relogios: r,
        historico: historico.clone(),
    };
    if let Ok(j) = serde_json::to_string_pretty(&p) {
        let _ = fs::File::create("estoque.json").and_then(|mut f| f.write_all(j.as_bytes()));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(5),
                        Constraint::Min(5),
                        Constraint::Length(5),
                        Constraint::Length(5),
                    ]
                    .as_ref(),
                )
                .split(size);

            let logo = r#" 
   ____      _          _     
  / ___| ___| | ___  __| |___ 
 | |  _ / _ \ |/ _ \/ _` / __|
 | |_| |  __/ |  __/ (_| \__ \
  \____|\___|_|\___|\__,_|___/
"#;

            let logo_par = Paragraph::new(logo).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_widget(logo_par, vertical_layout[0]);

            let horizontal_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(20), Constraint::Length(30)].as_ref())
                .split(vertical_layout[1]);

            let mut hotkeys_vec = vec![
                "Hotkeys:".to_string(),
                " [C] Cadastro".to_string(),
                " [B] Buscar".to_string(),
                " [H] Histórico (↑/↓ rola, ←/→ abas)".to_string(),
                " [G] Gráfico".to_string(),
                " [ENTER] Selecionar registro".to_string(),
                " [A] Adicionar/Comprar".to_string(),
                " [V] Vender".to_string(),
                " [ESC] Cancelar Seleção".to_string(),
                " [X] Sair".to_string(),
            ];

            if let Some(cod) = &app.chosen_relogio {
                for line in hotkeys_vec.iter_mut() {
                    if line.contains("[A]") && app.chosen_operation == Some('A') {
                        *line = format!(" [A] Adicionar/Comprar -> {} (Selecionado)", cod);
                    } else if line.contains("[V]") && app.chosen_operation == Some('V') {
                        *line = format!(" [V] Vender -> {} (Selecionado)", cod);
                    }
                }
            }

            // Historico: adicionar hotkey [P] Pesquisar
            if app.modo == Modo::Historico {
                hotkeys_vec.insert(5, " [P] Pesquisar Histórico".to_string());
            }

            let hotkeys_items: Vec<ListItem> = hotkeys_vec
                .iter()
                .map(|x| {
                    let mut style = Style::default();
                    if let Some(op) = app.chosen_operation {
                        if x.contains("[A]") && op == 'A' {
                            style = Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD);
                        } else if x.contains("[V]") && op == 'V' {
                            style = Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD);
                        }
                    }
                    if app.chosen_relogio.is_some() && x.contains("[ENTER] Selecionar registro") {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    ListItem::new(x.clone()).style(style)
                })
                .collect();
            let hotkeys_list = List::new(hotkeys_items)
                .block(Block::default().borders(Borders::ALL).title("Hotkeys"));
            f.render_widget(hotkeys_list, horizontal_layout[1]);

            let main_area = horizontal_layout[0];
            match app.modo {
                Modo::Estoques => {
                    let area = main_area;
                    let visible_height = area.height.saturating_sub(3) as usize;
                    let end = (app.estoques_offset + visible_height).min(app.estoques_list.len());
                    let visible_data = &app.estoques_list[app.estoques_offset..end];
                    let visible_rows = visible_data.iter().enumerate().map(|(i, r)| {
                        let real_index = app.estoques_offset + i;
                        let mut base_style = Style::default();
                        if real_index == app.estoques_selected {
                            base_style = base_style.bg(Color::White).fg(Color::Black);
                        }
                        if let Some(selected_cod) = &app.chosen_relogio {
                            if selected_cod == &r.codigo {
                                base_style = base_style.bg(Color::Yellow).fg(Color::Black);
                            }
                        }

                        Row::new(vec![
                            Cell::from(r.codigo.clone()),
                            Cell::from(r.quantidade.to_string()),
                        ])
                        .style(base_style)
                    });
                    let table = Table::new(
                        visible_rows,
                        &[Constraint::Percentage(70), Constraint::Percentage(30)],
                    )
                    .header(
                        Row::new(vec!["CÓDIGO", "QTD"]).style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    )
                    .block(Block::default().borders(Borders::ALL).title("Estoque"));
                    f.render_widget(table, area);
                }
                Modo::Historico => {
                    let titles = HistoricoTab::titles();
                    let tab_index = match app.historico_tab {
                        HistoricoTab::Todos => 0,
                        HistoricoTab::Compras => 1,
                        HistoricoTab::Vendas => 2,
                        HistoricoTab::Aquisicoes => 3,
                    };
                    let tab_titles: Vec<Span> = titles
                        .iter()
                        .enumerate()
                        .map(|(i, &t)| {
                            let style = if i == tab_index {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };
                            Span::styled(t, style)
                        })
                        .collect();
                    let tabs = Tabs::new(tab_titles)
                        .block(Block::default().borders(Borders::ALL).title("Filtros"))
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

                    let hist_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                        .split(main_area);

                    // Se estivermos editando (pesquisando histórico), mostrar input
                    let mut info = String::new();
                    if app.editing && app.modo == Modo::Historico {
                        info = format!("Filtrar histórico por código: {}", app.input);
                    } else {
                        info = "Pressione P para pesquisar no histórico".into();
                    }

                    let p = Paragraph::new(info).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Pesquisa no Histórico"),
                    );
                    f.render_widget(p, hist_layout[0]);

                    // Mostrar abas
                    f.render_widget(tabs, hist_layout[0]);

                    let data = app.get_historico_atual_vec();
                    let visible_height = hist_layout[1].height.saturating_sub(3) as usize;
                    let end = (app.historico_offset + visible_height).min(data.len());
                    let visible_data = &data[app.historico_offset..end];

                    let visible_rows = visible_data.iter().enumerate().map(|(i, h)| {
                        let real_index = app.historico_offset + i;
                        let oper_style = match h.operacao.as_str() {
                            "COMPRA" => Style::default().fg(Color::Green),
                            "VENDA" => Style::default().fg(Color::Red),
                            "CADASTRO" => Style::default().fg(Color::Yellow),
                            _ => Style::default().fg(Color::White),
                        };
                        let row_style = if real_index == app.historico_selected {
                            oper_style.add_modifier(Modifier::REVERSED)
                        } else {
                            oper_style
                        };
                        Row::new(vec![
                            Cell::from(h.timestamp.clone()),
                            Cell::from(h.operacao.clone()),
                            Cell::from(h.quantidade.to_string()),
                            Cell::from(h.codigo.clone()),
                        ])
                        .style(row_style)
                    });

                    let table = Table::new(
                        visible_rows,
                        &[
                            Constraint::Percentage(40),
                            Constraint::Percentage(20),
                            Constraint::Percentage(10),
                            Constraint::Percentage(30),
                        ],
                    )
                    .header(
                        Row::new(vec!["TIMESTAMP", "OPERACAO", "QTD", "CÓDIGO"]).style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    )
                    .block(Block::default().borders(Borders::ALL).title("Histórico"));

                    f.render_widget(table, hist_layout[1]);

                    // Se estiver editando a busca no histórico, mostrar sugestões
                    if app.editing && app.modo == Modo::Historico {
                        let suggest_area = {
                            let chunks = Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                                .split(hist_layout[1]);
                            chunks[0]
                        };

                        let suggestion_items: Vec<ListItem> = app
                            .historico_search_results
                            .iter()
                            .enumerate()
                            .map(|(i, (cod, dist))| {
                                let mut style = Style::default();
                                if i == app.historico_search_selected {
                                    style = style.bg(Color::White).fg(Color::Black);
                                }
                                ListItem::new(format!("{} (dist={})", cod, dist)).style(style)
                            })
                            .collect();

                        let suggestion_list = List::new(suggestion_items)
                            .block(Block::default().borders(Borders::ALL).title("Sugestões"));
                        f.render_widget(suggestion_list, suggest_area);
                    }
                }
                Modo::Cadastro => {
                    let titulo = "Cadastrar Relógio";
                    let instrucao = "Digite codigo quantidade p/cadastrar";
                    let cad_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                        .split(main_area);

                    let p = Paragraph::new(format!(
                        "{}: {}\nEnter p/confirmar cadastro se input.",
                        instrucao, app.input
                    ))
                    .block(Block::default().borders(Borders::ALL).title(titulo));
                    f.render_widget(p, cad_layout[0]);

                    let visible_height = cad_layout[1].height.saturating_sub(3) as usize;
                    let end = (app.cadastro_offset + visible_height).min(app.cadastro_list.len());
                    let visible_data = &app.cadastro_list[app.cadastro_offset..end];
                    let visible_rows = visible_data.iter().enumerate().map(|(i, r)| {
                        let real_index = app.cadastro_offset + i;
                        let mut base_style = Style::default();
                        if real_index == app.cadastro_selected {
                            base_style = base_style.bg(Color::White).fg(Color::Black);
                        }
                        if let Some(selected_cod) = &app.chosen_relogio {
                            if selected_cod == &r.codigo {
                                base_style = base_style.bg(Color::Yellow).fg(Color::Black);
                            }
                        }
                        Row::new(vec![
                            Cell::from(r.codigo.clone()),
                            Cell::from(r.quantidade.to_string()),
                        ])
                        .style(base_style)
                    });
                    let table = Table::new(
                        visible_rows,
                        &[Constraint::Percentage(70), Constraint::Percentage(30)],
                    )
                    .header(
                        Row::new(vec!["CÓDIGO", "QTD"]).style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Relógios Cadastrados"),
                    );
                    f.render_widget(table, cad_layout[1]);
                }
                Modo::Buscar => {
                    let titulo = "Buscar Relógio";
                    let instrucao = "Digite código p/buscar, selecione resultado e Enter p/opções";
                    let search_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
                        .split(main_area);

                    let p =
                        Paragraph::new(format!("{}: {}\nESC p/ cancelar", instrucao, app.input))
                            .block(Block::default().borders(Borders::ALL).title(titulo));
                    f.render_widget(p, search_layout[0]);

                    if !app.input.is_empty() {
                        let visible_height = search_layout[1].height.saturating_sub(3) as usize;
                        let end =
                            (app.buscar_offset + visible_height).min(app.buscar_results.len());
                        let visible_data = &app.buscar_results[app.buscar_offset..end];

                        let visible_rows =
                            visible_data
                                .iter()
                                .enumerate()
                                .map(|(i, (cod, qtd, dist))| {
                                    let real_index = app.buscar_offset + i;
                                    let mut base_style = Style::default();
                                    if real_index == app.buscar_selected {
                                        base_style = base_style.bg(Color::White).fg(Color::Black);
                                    }
                                    if let Some(selected_cod) = &app.chosen_relogio {
                                        if selected_cod == cod {
                                            base_style =
                                                base_style.bg(Color::Yellow).fg(Color::Black);
                                        }
                                    }
                                    Row::new(vec![
                                        Cell::from(cod.clone()),
                                        Cell::from(qtd.to_string()),
                                        Cell::from(dist.to_string()),
                                    ])
                                    .style(base_style)
                                });
                        let table = Table::new(
                            visible_rows,
                            &[
                                Constraint::Percentage(50),
                                Constraint::Percentage(20),
                                Constraint::Percentage(30),
                            ],
                        )
                        .header(
                            Row::new(vec!["CÓDIGO", "QTD", "DIST"]).style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        )
                        .block(Block::default().borders(Borders::ALL).title("Resultados"));
                        f.render_widget(table, search_layout[1]);
                    } else {
                        let info = Paragraph::new("Digite algo para buscar.")
                            .block(Block::default().borders(Borders::ALL).title("Resultados"));
                        f.render_widget(info, search_layout[1]);
                    }
                }
                Modo::Grafico => {
                    let dia_data = app.agrupamento_por_dia();

                    // Monta os dados em formato (&str, u64) para o BarChart
                    let vendas_data: Vec<(&str, u64)> = dia_data
                        .iter()
                        .map(|(d, v, _c)| (d.as_str(), *v as u64))
                        .collect();
                    let compras_data: Vec<(&str, u64)> = dia_data
                        .iter()
                        .map(|(d, _v, c)| (d.as_str(), *c as u64))
                        .collect();

                    let graf_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(main_area);
                    let vendas_chart = ratatui::widgets::BarChart::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Vendas (Últimos 7 dias)"),
                        )
                        .data(&vendas_data)
                        .bar_width(5)
                        .bar_style(Style::default().fg(Color::Red))
                        .value_style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        );

                    let compras_chart = ratatui::widgets::BarChart::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Compras (Últimos 7 dias)"),
                        )
                        .data(&compras_data)
                        .bar_width(5)
                        .bar_style(Style::default().fg(Color::Green))
                        .value_style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        );

                    f.render_widget(vendas_chart, graf_layout[0]);
                    f.render_widget(compras_chart, graf_layout[1]);
                }
                Modo::Compra => {
                    let instrucao = "Digite codigo quantidade, Enter p/ confirmar, Esc p/ cancelar";
                    let p = Paragraph::new(format!("{}: {}\n", instrucao, app.input)).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Adicionar Estoque"),
                    );
                    f.render_widget(p, main_area);
                }
                Modo::Venda => {
                    let instrucao = "Digite codigo quantidade, Enter p/ confirmar, Esc p/ cancelar";
                    let p = Paragraph::new(format!("{}: {}\n", instrucao, app.input)).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Vender Relógio"),
                    );
                    f.render_widget(p, main_area);
                }
            }

            let logs_area = vertical_layout[2];
            let qtd_logs = 5;
            let total_msg = app.mensagens.len();
            let start_log = if total_msg > qtd_logs {
                total_msg - qtd_logs
            } else {
                0
            };
            let visible_logs = &app.mensagens[start_log..];
            let logs_items: Vec<ListItem> = visible_logs
                .iter()
                .map(|m| ListItem::new(m.as_str()))
                .collect();
            let lista_logs = List::new(logs_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Logs (Últimas Mensagens)"),
            );
            f.render_widget(lista_logs, logs_area);

            let msgs_area = vertical_layout[3];
            let msgs: Vec<ListItem> = app
                .mensagens
                .iter()
                .map(|m| ListItem::new(m.as_str()))
                .collect();
            let rodape = List::new(msgs).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Mensagens (Histórico Completo)"),
            );
            f.render_widget(rodape, msgs_area);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    if app.editing {
                        match app.modo {
                            Modo::Cadastro => match k.code {
                                KeyCode::Enter => {
                                    let parts: Vec<&str> =
                                        app.input.trim().split_whitespace().collect();
                                    if parts.len() == 2 {
                                        if let Ok(qtd) = parts[1].parse::<i32>() {
                                            app.cadastrar_relogio(parts[0].to_string(), qtd);
                                        } else {
                                            app.mensagens.push("Quantidade inválida!".into());
                                        }
                                    }
                                    app.sai_modo_insercao();
                                }
                                KeyCode::Esc => {
                                    app.sai_modo_insercao();
                                }
                                KeyCode::Backspace => {
                                    app.input.pop();
                                }
                                KeyCode::Char(ch) => {
                                    app.input.push(ch);
                                }
                                _ => {}
                            },
                            Modo::Buscar => match k.code {
                                KeyCode::Enter => {
                                    app.editing = false;
                                }
                                KeyCode::Esc => {
                                    app.sai_modo_insercao();
                                }
                                KeyCode::Backspace => {
                                    app.input.pop();
                                    app.atualizar_busca_results();
                                }
                                KeyCode::Char(ch) => {
                                    app.input.push(ch);
                                    app.atualizar_busca_results();
                                }
                                _ => {}
                            },
                            Modo::Historico => {
                                // Editando filtro no histórico
                                match k.code {
                                    KeyCode::Enter => {
                                        // Ao apertar Enter, filtra pelo código selecionado
                                        if !app.historico_search_results.is_empty() {
                                            let (cod, _) = app.historico_search_results
                                                [app.historico_search_selected]
                                                .clone();
                                            app.filtrar_historico(&cod);
                                        } else {
                                            // Se não há resultados, filtra pelo input
                                            let cod = app.input.trim().to_string();
                                            app.filtrar_historico(&cod);
                                        }
                                        app.editing = false;
                                        app.input.clear();
                                    }
                                    KeyCode::Esc => {
                                        // Cancela filtro
                                        app.input.clear();
                                        app.editing = false;
                                        app.filtrar_historico("");
                                    }
                                    KeyCode::Backspace => {
                                        app.input.pop();
                                        app.atualizar_historico_search_results();
                                    }
                                    KeyCode::Char(ch) => {
                                        app.input.push(ch);
                                        app.atualizar_historico_search_results();
                                    }
                                    KeyCode::Up => {
                                        // Navega nas sugestões
                                        app.historico_search_up();
                                    }
                                    KeyCode::Down => {
                                        app.historico_search_down();
                                    }
                                    _ => {}
                                }
                            }
                            Modo::Compra => match k.code {
                                KeyCode::Enter => {
                                    let parts: Vec<&str> =
                                        app.input.trim().split_whitespace().collect();
                                    if parts.len() == 2 {
                                        if let Ok(qtd) = parts[1].parse::<i32>() {
                                            app.comprar_relogio(parts[0].to_string(), qtd);
                                        } else {
                                            app.mensagens.push("Quantidade inválida!".into());
                                        }
                                    } else {
                                        app.mensagens
                                            .push("Formato incorreto. codigo quantidade".into());
                                    }
                                    app.modo = Modo::Estoques;
                                    app.editing = false;
                                    app.input.clear();
                                    app.chosen_relogio = None;
                                    app.chosen_operation = None;
                                }
                                KeyCode::Esc => {
                                    app.modo = Modo::Estoques;
                                    app.editing = false;
                                    app.input.clear();
                                    app.chosen_relogio = None;
                                    app.chosen_operation = None;
                                }
                                KeyCode::Backspace => {
                                    app.input.pop();
                                }
                                KeyCode::Char(ch) => {
                                    app.input.push(ch);
                                }
                                _ => {}
                            },
                            Modo::Venda => match k.code {
                                KeyCode::Enter => {
                                    let parts: Vec<&str> =
                                        app.input.trim().split_whitespace().collect();
                                    if parts.len() == 2 {
                                        if let Ok(qtd) = parts[1].parse::<i32>() {
                                            app.vender_relogio(parts[0].to_string(), qtd);
                                        } else {
                                            app.mensagens.push("Quantidade inválida!".into());
                                        }
                                    } else {
                                        app.mensagens
                                            .push("Formato incorreto. codigo quantidade".into());
                                    }
                                    app.modo = Modo::Estoques;
                                    app.editing = false;
                                    app.input.clear();
                                    app.chosen_relogio = None;
                                    app.chosen_operation = None;
                                }
                                KeyCode::Esc => {
                                    app.modo = Modo::Estoques;
                                    app.editing = false;
                                    app.input.clear();
                                    app.chosen_relogio = None;
                                    app.chosen_operation = None;
                                }
                                KeyCode::Backspace => {
                                    app.input.pop();
                                }
                                KeyCode::Char(ch) => {
                                    app.input.push(ch);
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    } else {
                        match k.code {
                            KeyCode::Char('x') => {
                                break;
                            }
                            KeyCode::Esc => {
                                app.cancelar_selecao();
                                app.modo = Modo::Estoques;
                                app.historico_filtrado = None;
                                app.input.clear();
                                app.editing = false;
                            }
                            KeyCode::Char('c') => {
                                app.entra_modo_insercao(Modo::Cadastro);
                            }
                            KeyCode::Char('b') => {
                                app.entra_modo_insercao(Modo::Buscar);
                                app.atualizar_busca_results();
                            }
                            KeyCode::Char('h') => {
                                if app.modo != Modo::Historico {
                                    app.modo = Modo::Historico;
                                    app.mensagens.push("Histórico: ↑/↓ rola, ←/→ abas, P p/pesquisar, Enter p/filtrar, ESC p/sair.".into());
                                    app.editing = false;
                                    app.input.clear();
                                    app.historico_offset = 0;
                                    app.historico_selected = 0;
                                    // Ao entrar no modo histórico, não estamos editando ainda.
                                }
                            }
                            KeyCode::Char('g') => {
                                app.modo = Modo::Grafico;
                            }
                            KeyCode::Left => {
                                if app.modo == Modo::Historico {
                                    app.historico_tab_prev();
                                }
                            }
                            KeyCode::Right => {
                                if app.modo == Modo::Historico {
                                    app.historico_tab_next();
                                }
                            }
                            KeyCode::Up => match app.modo {
                                Modo::Estoques => {
                                    if app.estoques_selected > 0 {
                                        app.estoques_selected -= 1;
                                        if app.estoques_selected < app.estoques_offset {
                                            app.estoques_offset = app.estoques_selected;
                                        }
                                    }
                                }
                                Modo::Historico => {
                                    app.historico_select_up();
                                }
                                Modo::Cadastro => {
                                    app.cadastro_select_up();
                                }
                                Modo::Buscar => {
                                    app.buscar_select_up();
                                }
                                _ => {}
                            },
                            KeyCode::Down => match app.modo {
                                Modo::Estoques => {
                                    if app.estoques_selected + 1 < app.estoques_list.len() {
                                        app.estoques_selected += 1;
                                        let vis_height = 5;
                                        if app.estoques_selected >= app.estoques_offset + vis_height
                                        {
                                            app.estoques_offset =
                                                app.estoques_selected - vis_height + 1;
                                        }
                                    }
                                }
                                Modo::Historico => {
                                    app.historico_select_down();
                                }
                                Modo::Cadastro => {
                                    app.cadastro_select_down();
                                }
                                Modo::Buscar => {
                                    app.buscar_select_down();
                                }
                                _ => {}
                            },
                            KeyCode::Enter => match app.modo {
                                Modo::Cadastro => {
                                    let parts: Vec<&str> =
                                        app.input.trim().split_whitespace().collect();
                                    if parts.len() == 2 {
                                        if let Ok(qtd) = parts[1].parse::<i32>() {
                                            app.cadastrar_relogio(parts[0].to_string(), qtd);
                                            app.input.clear();
                                        }
                                    }
                                }
                                Modo::Buscar => {
                                    if !app.buscar_results.is_empty() {
                                        let (cod, _, _) =
                                            app.buscar_results[app.buscar_selected].clone();
                                        app.selecionar_registro(cod);
                                    }
                                }
                                Modo::Estoques => {
                                    if let Some(r) = app.estoques_list.get(app.estoques_selected) {
                                        app.selecionar_registro(r.codigo.clone());
                                    }
                                }
                                _ => {}
                            },
                            KeyCode::Char('A') | KeyCode::Char('a') => {
                                if app.chosen_relogio.is_some()
                                    && (app.modo == Modo::Estoques || app.modo == Modo::Buscar)
                                {
                                    app.escolher_operacao('A');
                                }
                            }
                            KeyCode::Char('V') | KeyCode::Char('v') => {
                                if app.chosen_relogio.is_some()
                                    && (app.modo == Modo::Estoques || app.modo == Modo::Buscar)
                                {
                                    app.escolher_operacao('V');
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                // Apertar P no histórico para pesquisar
                                if app.modo == Modo::Historico && !app.editing {
                                    app.editing = true;
                                    app.input.clear();
                                    app.atualizar_historico_search_results();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
