use super::*;

#[derive(Debug)]
pub struct Document {
  content: Rope,
  _language: String,
  _version: i32,
  tree: Option<Tree>,
}

impl Document {
  pub(crate) fn from_params(params: lsp::DidOpenTextDocumentParams) -> Self {
    let document = params.text_document;

    let mut doc = Self {
      content: Rope::from_str(&document.text),
      _language: document.language_id,
      _version: document.version,
      tree: None,
    };

    doc.parse();

    doc
  }

  pub(crate) fn parse(&mut self) {
    let mut parser = Parser::new();

    unsafe {
      parser
        .set_language(&tree_sitter_just())
        .expect("Failed to load `tree-sitter-just`");
    }

    let text = self.content.to_string();

    self.tree = parser.parse(&text, None);
  }

  pub(crate) fn apply_change(
    &mut self,
    params: lsp::DidChangeTextDocumentParams,
  ) -> Result {
    let edits = params
      .content_changes
      .iter()
      .map(|change| self.content.build_edit(change))
      .collect::<Result<Vec<_>, _>>()?;

    for edit in edits {
      self.content.apply_edit(&edit);
    }

    self.parse();

    Ok(())
  }

  pub(crate) fn position_to_point(&self, position: lsp::Position) -> Point {
    Point {
      row: position.line as usize,
      column: position.character as usize,
    }
  }

  pub(crate) fn point_to_position(&self, point: Point) -> lsp::Position {
    lsp::Position {
      line: point.row as u32,
      character: point.column as u32,
    }
  }

  pub(crate) fn node_to_range(&self, node: &Node) -> lsp::Range {
    lsp::Range {
      start: self.point_to_position(node.start_position()),
      end: self.point_to_position(node.end_position()),
    }
  }

  pub(crate) fn node_at_position(
    &self,
    position: lsp::Position,
  ) -> Option<Node> {
    if let Some(tree) = &self.tree {
      let point = self.position_to_point(position);
      Some(tree.root_node().descendant_for_point_range(point, point)?)
    } else {
      None
    }
  }

  pub(crate) fn find_nodes(&self, kind: &str) -> Vec<Node> {
    let mut nodes = Vec::new();

    if let Some(tree) = &self.tree {
      let mut cursor = tree.root_node().walk();
      self.collect_nodes(&mut cursor, kind, &mut nodes);
    }

    nodes
  }

  fn collect_nodes<'a>(
    &self,
    cursor: &mut tree_sitter::TreeCursor<'a>,
    kind: &str,
    nodes: &mut Vec<Node<'a>>,
  ) {
    let node = cursor.node();

    if node.kind() == kind {
      nodes.push(node);
    }

    if cursor.goto_first_child() {
      loop {
        self.collect_nodes(cursor, kind, nodes);

        if !cursor.goto_next_sibling() {
          break;
        }
      }

      cursor.goto_parent();
    }
  }

  pub(crate) fn get_node_text(&self, node: &Node) -> String {
    self
      .content
      .slice(
        self.content.byte_to_char(node.start_byte())
          ..self.content.byte_to_char(node.end_byte()),
      )
      .to_string()
  }

  pub(crate) fn find_recipe_by_name<'a>(
    &'a self,
    name: &str,
  ) -> Option<tree_sitter::Node<'a>> {
    let recipe_nodes = self.find_nodes("recipe");

    for recipe_node in recipe_nodes {
      if let Some(recipe_header) =
        self.find_child_by_kind(&recipe_node, "recipe_header")
      {
        if let Some(identifier) =
          self.find_child_by_kind(&recipe_header, "identifier")
        {
          let recipe_name = self.get_node_text(&identifier);

          if recipe_name == name {
            return Some(recipe_node);
          }
        }
      }
    }

    None
  }

  fn find_child_by_kind<'a>(
    &'a self,
    node: &'a tree_sitter::Node,
    kind: &str,
  ) -> Option<tree_sitter::Node<'a>> {
    for i in 0..node.child_count() {
      if let Some(child) = node.child(i) {
        if child.kind() == kind {
          return Some(child);
        }
      }
    }

    None
  }

  pub(crate) fn find_all_recipe_references(
    &self,
    recipe_name: &str,
    uri: &lsp::Url,
  ) -> Vec<lsp::Location> {
    let mut locations = Vec::new();

    let dependency_nodes = self.find_nodes("dependency");

    for dependency_node in dependency_nodes {
      if let Some(identifier) =
        self.find_child_by_kind(&dependency_node, "identifier")
      {
        let dep_name = self.get_node_text(&identifier);

        if dep_name == recipe_name {
          locations.push(lsp::Location {
            uri: uri.clone(),
            range: self.node_to_range(&identifier),
          });
        }
      }
    }

    let alias_nodes = self.find_nodes("alias");

    for alias_node in alias_nodes {
      if let Some(right) = self.find_child_by_kind(&alias_node, "right") {
        if let Some(identifier) = self.find_child_by_kind(&right, "identifier")
        {
          let alias_target = self.get_node_text(&identifier);

          if alias_target == recipe_name {
            locations.push(lsp::Location {
              uri: uri.clone(),
              range: self.node_to_range(&identifier),
            });
          }
        }
      }
    }

    locations
  }
}
