//! Graphviz SVG → Dioxus renderer (router-optional).
//!
//! Internal link interception only happens if a Navigator context is present (i.e. we are inside a Router).
//! Otherwise internal links are rendered as ordinary <a href="..."> elements.
//!
//! Unknown attributes are appended as CSS custom properties into `style` to avoid losing data.
use dioxus::prelude::*;
use dioxus_router::Navigator;
use roxmltree::{Document, Node};
use std::borrow::Cow;

// Namespace constant for xlink
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

// ------------------------- Link classification -------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum LinkKind {
    Internal(String),
    External(String),
    Fragment(String),
    None,
}

// ------------------------- Config -------------------------

#[derive(Clone)]
pub struct SvgBuildConfig {
    pub classify_link: fn(&str) -> LinkKind,
    pub map_internal_route: Option<fn(&str) -> Option<String>>,
    pub on_fragment_click: Option<fn(&str)>,
    pub on_title: Option<fn(&str)>,
    pub strip_doctype: bool,
}

impl PartialEq for SvgBuildConfig {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Default for SvgBuildConfig {
    fn default() -> Self {
        SvgBuildConfig {
            classify_link: |href: &str| {
                if let Some(rest) = href.strip_prefix('#') {
                    LinkKind::Fragment(rest.to_string())
                } else if href.starts_with("http://") || href.starts_with("https://") {
                    LinkKind::External(href.to_string())
                } else if href.starts_with('/') {
                    LinkKind::Internal(href.to_string())
                } else {
                    LinkKind::None
                }
            },
            map_internal_route: None,
            on_fragment_click: None,
            on_title: None,
            strip_doctype: true,
        }
    }
}

// ------------------------- Attribute collection -------------------------

#[derive(Default)]
struct SvgAttrs {
    id: Option<String>,
    class: Option<String>,
    style: Option<String>,
    transform: Option<String>,
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<String>,
    stroke_dasharray: Option<String>,
    font_size: Option<String>,
    font_family: Option<String>,
    font_weight: Option<String>,
    text_anchor: Option<String>,
    xml_space: Option<String>,

    x: Option<String>,
    y: Option<String>,
    dx: Option<String>,
    dy: Option<String>,
    cx: Option<String>,
    cy: Option<String>,
    rx: Option<String>,
    ry: Option<String>,
    r: Option<String>,
    width: Option<String>,
    height: Option<String>,
    d: Option<String>,
    points: Option<String>,
    view_box: Option<String>,

    href: Option<String>,        // plain href
    xlink_href: Option<String>,  // namespaced href
    xlink_title: Option<String>, // tooltip
    target: Option<String>,
    rel: Option<String>,

    // For unknown attributes (debug)
    extra: Vec<(String, String)>,
}

fn collect_attrs(node: Node) -> SvgAttrs {
    let mut sa = SvgAttrs::default();
    for a in node.attributes() {
        let ns = a.namespace();
        let local = a.name();
        let value = a.value().to_string();

        match (ns, local) {
            (Some(XLINK_NS), "href") => sa.xlink_href = Some(value),
            (Some(XLINK_NS), "title") => sa.xlink_title = Some(value),

            (None, "id") => sa.id = Some(value),
            (None, "class") => sa.class = Some(value),
            (None, "style") => sa.style = Some(value),
            (None, "transform") => sa.transform = Some(value),
            (None, "fill") => sa.fill = Some(value),
            (None, "stroke") => sa.stroke = Some(value),
            (None, "stroke-width") => sa.stroke_width = Some(value),
            (None, "stroke-dasharray") => sa.stroke_dasharray = Some(value),
            (None, "font-size") => sa.font_size = Some(value),
            (None, "font-family") => sa.font_family = Some(value),
            (None, "font-weight") => sa.font_weight = Some(value),
            (None, "text-anchor") => sa.text_anchor = Some(value),
            (None, "xml:space") => sa.xml_space = Some(value),

            (None, "x") => sa.x = Some(value),
            (None, "y") => sa.y = Some(value),
            (None, "dx") => sa.dx = Some(value),
            (None, "dy") => sa.dy = Some(value),
            (None, "cx") => sa.cx = Some(value),
            (None, "cy") => sa.cy = Some(value),
            (None, "rx") => sa.rx = Some(value),
            (None, "ry") => sa.ry = Some(value),
            (None, "r") => sa.r = Some(value),
            (None, "width") => sa.width = Some(value),
            (None, "height") => sa.height = Some(value),
            (None, "d") => sa.d = Some(value),
            (None, "points") => sa.points = Some(value),
            (None, "viewBox") => sa.view_box = Some(value),

            (None, "href") => sa.href = Some(value),
            (None, "target") => sa.target = Some(value),
            (None, "rel") => sa.rel = Some(value),

            _ => {
                // Preserve unknown for debugging (not converted into CSS semantics).
                let key = match ns {
                    Some(ns_uri) => format!("{ns_uri}:{local}"),
                    None => local.to_string(),
                };
                sa.extra.push((key, value));
            }
        }
    }
    sa
}

// ------------------------- Sanitization (DTD strip) -------------------------

fn strip_doctype(raw: &str) -> Cow<'_, str> {
    if !raw.contains("<!DOCTYPE") {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    let b = raw.as_bytes();
    let mut removed = false;
    while i < b.len() {
        if b[i] == b'<' && raw[i..].starts_with("<!DOCTYPE") {
            removed = true;
            i += "<!DOCTYPE".len();
            while i < b.len() && b[i] != b'>' {
                i += 1;
            }
            if i < b.len() {
                i += 1;
            }
            while i < b.len() && matches!(b[i], b'\n' | b'\r') {
                i += 1;
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    if removed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(raw)
    }
}

// ------------------------- Top-level component -------------------------

#[component]
pub fn GraphvizSvg(svg_text: String, config: SvgBuildConfig) -> Element {
    let navigator = use_context::<Option<Navigator>>();

    let mut cow: Cow<'_, str> = if config.strip_doctype {
        strip_doctype(&svg_text)
    } else {
        Cow::Borrowed(svg_text.as_str())
    };

    let doc = loop {
        match Document::parse(&cow) {
            Ok(d) => break d,
            Err(e) => {
                let did_strip = !matches!(cow, Cow::Borrowed(_));
                if !did_strip && svg_text.contains("<!DOCTYPE") {
                    cow = strip_doctype(&svg_text);
                    continue;
                } else {
                    return render_parse_error(e, did_strip || config.strip_doctype);
                }
            }
        }
    };

    let Some(root) = doc.descendants().find(|n| n.has_tag_name("svg")) else {
        return rsx! { svg { class: "graphviz-svg error", "No <svg> root found." } };
    };

    build_node(root, &config, navigator.as_ref()).unwrap_or(rsx! {})
}

fn render_parse_error(err: roxmltree::Error, did_strip: bool) -> Element {
    rsx! {
        svg {
            class: "graphviz-svg error",
            style: "padding:8px;font-family:monospace;font-size:12px;fill:#900;",
            "SVG parse error (strip_doctype={did_strip}): {err}"
        }
    }
}

// ------------------------- Recursion -------------------------

fn build_node(node: Node, cfg: &SvgBuildConfig, navigator: Option<&Navigator>) -> Option<Element> {
    if node.is_text() {
        let t = node.text().unwrap_or_default();
        if t.trim().is_empty() {
            return None;
        }
        return Some(rsx! { "{t}" });
    }
    if !node.is_element() {
        return None;
    }

    let tag = node.tag_name().name();
    let attrs = collect_attrs(node);
    let children: Vec<Element> = node
        .children()
        .filter_map(|c| build_node(c, cfg, navigator))
        .collect();

    let el = match tag {
        "svg" => rsx! {
            svg {
                id: attrs.id,
                class: attrs.class,
                view_box: attrs.view_box,
                style: attrs.style,
                for child in children { {child} }
            }
        },
        "g" => rsx! {
            g {
                id: attrs.id,
                class: attrs.class,
                transform: attrs.transform,
                style: attrs.style,
                for child in children { {child} }
            }
        },
        "text" => rsx! {
            text {
                id: attrs.id,
                class: attrs.class,
                x: attrs.x,
                y: attrs.y,
                dx: attrs.dx,
                dy: attrs.dy,
                fill: attrs.fill,
                font_size: attrs.font_size,
                font_family: attrs.font_family,
                "font-weight": attrs.font_weight,
                text_anchor: attrs.text_anchor,
                "xml:space": attrs.xml_space,
                style: attrs.style,
                for child in children { {child} }
            }
        },
        "title" => {
            // Pass through <title>
            if let Some(t) = node.text() {
                if let Some(cb) = cfg.on_title {
                    cb(t);
                }
                rsx! { title { "{t}" } }
            } else {
                rsx! { title { for child in children { {child} } } }
            }
        }
        "ellipse" => rsx! {
            ellipse {
                id: attrs.id,
                class: attrs.class,
                cx: attrs.cx,
                cy: attrs.cy,
                rx: attrs.rx,
                ry: attrs.ry,
                r: attrs.r,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
            }
        },
        "circle" => rsx! {
            circle {
                id: attrs.id,
                class: attrs.class,
                cx: attrs.cx,
                cy: attrs.cy,
                r: attrs.r,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
            }
        },
        "rect" => rsx! {
            rect {
                id: attrs.id,
                class: attrs.class,
                x: attrs.x,
                y: attrs.y,
                width: attrs.width,
                height: attrs.height,
                rx: attrs.rx,
                ry: attrs.ry,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
                for child in children { {child} }
            }
        },
        "polygon" => rsx! {
            polygon {
                id: attrs.id,
                class: attrs.class,
                points: attrs.points,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
            }
        },
        "polyline" => rsx! {
            polyline {
                id: attrs.id,
                class: attrs.class,
                points: attrs.points,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
            }
        },
        "path" => rsx! {
            path {
                id: attrs.id,
                class: attrs.class,
                d: attrs.d,
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
                "stroke-dasharray": attrs.stroke_dasharray,
                style: attrs.style,
            }
        },
        "a" => build_anchor(attrs, children, cfg, navigator),
        _ => {
            // Unknown tag -> wrap for debugging
            rsx! {
                g {
                    id: attrs.id,
                    class: attrs.class,
                    style: attrs.style,
                    "data-unknown-tag": tag,
                    for child in children { {child} }
                }
            }
        }
    };

    Some(el)
}

// ------------------------- Anchor -------------------------

fn build_anchor(
    a: SvgAttrs,
    children: Vec<Element>,
    cfg: &SvgBuildConfig,
    navigator: Option<&Navigator>,
) -> Element {
    // Effective hyperlink
    let mut effective_href = a.href.clone().or(a.xlink_href.clone());

    if let Some(mapper) = cfg.map_internal_route.as_ref() {
        if let Some(href) = &effective_href {
            if let Some(mapped) = mapper(href) {
                effective_href = Some(mapped);
            }
        }
    }

    // Determine if a <title> child already exists
    let has_title_child = children.iter().any(|el| {
        // Cheap check by rendering to string would be heavy; instead rely on partial introspection:
        // We can't introspect easily without pattern matching VNode; assume none for now.
        // Provide a config knob if needed later.
        false
    });

    // Optional tooltip <title> from xlink:title
    let tooltip_node = if !has_title_child {
        a.xlink_title.as_ref().map(|t| rsx! { title { "{t}" } })
    } else {
        None
    };

    match effective_href {
        Some(href) => {
            let kind = (cfg.classify_link)(&href);
            match kind {
                LinkKind::External(url) => {
                    rsx! {
                        a {
                            id: a.id,
                            class: a.class,
                            href: "{url}",
                            "xlink:href": a.xlink_href,
                            "xlink:title": a.xlink_title,
                            target: a.target.or(Some("_blank".into())),
                            rel: a.rel.or(Some("noopener noreferrer".into())),
                            style: a.style,
                            { tooltip_node }
                            for child in children { {child} }
                        }
                    }
                }
                LinkKind::Internal(route) => {
                    let route_owned = route.clone();
                    rsx! {
                        a {
                            id: a.id,
                            class: a.class,
                            href: "{route_owned}",
                            "xlink:href": a.xlink_href,
                            "xlink:title": a.xlink_title,
                            style: a.style,
                            onclick: {
                                let navigator = navigator.cloned();
                                    move |evt| {
                                    if let Some(nav) = navigator {
                                        evt.prevent_default();
                                        nav.push(route_owned.as_str());
                                    }
                                }
                            },
                            { tooltip_node }
                            for child in children { {child} }
                        }
                    }
                }
                LinkKind::Fragment(id) => {
                    let id_owned = id.clone();
                    let cb = cfg.on_fragment_click;
                    rsx! {
                        a {
                            id: a.id,
                            class: a.class,
                            href: "#{id_owned}",
                            "xlink:title": a.xlink_title,
                            style: a.style,
                            onclick: move |evt| {
                                evt.prevent_default();
                                if let Some(f) = cb {
                                    f(&id_owned);
                                }
                            },
                            { tooltip_node }
                            for child in children { {child} }
                        }
                    }
                }
                LinkKind::None => {
                    rsx! {
                        a {
                            id: a.id,
                            class: a.class,
                            href: href,
                            "xlink:href": a.xlink_href,
                            "xlink:title": a.xlink_title,
                            style: a.style,
                            { tooltip_node }
                            for child in children { {child} }
                        }
                    }
                }
            }
        }
        None => {
            // Tooltip only wrapper (cluster anchors sometimes)
            rsx! {
                a {
                    id: a.id,
                    class: a.class,
                    "xlink:title": a.xlink_title,
                    style: a.style,
                    { tooltip_node }
                    for child in children { {child} }
                }
            }
        }
    }
}

// ------------------------- Utilities -------------------------

fn attribute_name(attr: roxmltree::Attribute) -> String {
    match (attr.namespace(), attr.name()) {
        (Some(ns), local) => format!("{ns}:{local}"),
        (None, local) => local.to_string(),
    }
}
