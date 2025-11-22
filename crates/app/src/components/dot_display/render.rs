//! Graphviz SVG → Dioxus renderer (router-optional).
//!
//! Internal link interception only happens if a Navigator context is present (i.e. we are inside a Router).
//! Otherwise internal links are rendered as ordinary <a href="..."> elements.
//!
//! Unknown attributes are appended as CSS custom properties into `style` to avoid losing data.

use dioxus::prelude::*;
use dioxus_logger::tracing;
use dioxus_router::Navigator;
use roxmltree::{Document, Node}; // for optional context

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
}

impl PartialEq for SvgBuildConfig {
    fn eq(&self, _other: &Self) -> bool {
        // Avoid function pointer equality warnings
        true
    }
}

impl Default for SvgBuildConfig {
    fn default() -> Self {
        SvgBuildConfig {
            classify_link: |href: &str| {
                if let Some(stripped) = href.strip_prefix('#') {
                    LinkKind::Fragment(stripped.to_string())
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
        }
    }
}

// ------------------------- Top-level component -------------------------

#[component]
pub fn GraphvizSvg(svg_text: String, config: SvgBuildConfig) -> Element {
    // Try to get a Navigator without panicking; returns Option<&Navigator>
    let navigator = use_context::<Option<Navigator>>();

    tracing::info!(
        "Rendering Graphviz SVG with navigator: {:?}",
        navigator.is_some()
    );

    let doc = match Document::parse(&svg_text) {
        Ok(d) => d,
        Err(err) => {
            tracing::error!("SVG parse error: {}", err);
            return rsx! {
                svg { class: "graphviz-svg error", "SVG parse error: {err}" }
            };
        }
    };

    let Some(root) = doc.descendants().find(|n| n.has_tag_name("svg")) else {
        tracing::error!("No svg found in root");
        return rsx! { svg { class: "graphviz-svg empty", "No <svg> root found." } };
    };

    build_node(root, &config, navigator.as_ref()).unwrap_or(rsx! {})
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
    font_size: Option<String>,
    font_family: Option<String>,
    text_anchor: Option<String>,
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
    href: Option<String>,
    target: Option<String>,
    rel: Option<String>,
    extra_style: Vec<(String, String)>,
}

fn collect_attrs(node: Node) -> SvgAttrs {
    let mut sa = SvgAttrs::default();
    for a in node.attributes() {
        let name = attribute_name(a);
        let value = a.value().to_string();
        match name.as_str() {
            "id" => sa.id = Some(value),
            "class" => sa.class = Some(value),
            "style" => sa.style = Some(value),
            "transform" => sa.transform = Some(value),
            "fill" => sa.fill = Some(value),
            "stroke" => sa.stroke = Some(value),
            "stroke-width" => sa.stroke_width = Some(value),
            "font-size" => sa.font_size = Some(value),
            "font-family" => sa.font_family = Some(value),
            "text-anchor" => sa.text_anchor = Some(value),
            "x" => sa.x = Some(value),
            "y" => sa.y = Some(value),
            "dx" => sa.dx = Some(value),
            "dy" => sa.dy = Some(value),
            "cx" => sa.cx = Some(value),
            "cy" => sa.cy = Some(value),
            "rx" => sa.rx = Some(value),
            "ry" => sa.ry = Some(value),
            "r" => sa.r = Some(value),
            "width" => sa.width = Some(value),
            "height" => sa.height = Some(value),
            "d" => sa.d = Some(value),
            "points" => sa.points = Some(value),
            "viewBox" => sa.view_box = Some(value),
            "href" | "xlink:href" => sa.href = Some(value),
            "target" => sa.target = Some(value),
            "rel" => sa.rel = Some(value),
            _ => sa.extra_style.push((name, value)),
        }
    }
    if !sa.extra_style.is_empty() {
        let mut merged = sa.style.unwrap_or_default();
        for (k, v) in &sa.extra_style {
            merged.push_str(&format!("--{}:{};", k.replace(':', "_"), v));
        }
        sa.style = Some(merged);
    }
    sa
}

// ------------------------- Recursion -------------------------

fn build_node(node: Node, cfg: &SvgBuildConfig, navigator: Option<&Navigator>) -> Option<Element> {
    tracing::info!("Building node: {:?}", node.tag_name().name());
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
                style: attrs.style,
                view_box: attrs.view_box,
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
                text_anchor: attrs.text_anchor,
                style: attrs.style,
                for child in children { {child} }
            }
        },
        "title" => {
            if let Some(t) = node.text() {
                if let Some(on_title) = cfg.on_title {
                    on_title(t);
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
                fill: attrs.fill,
                stroke: attrs.stroke,
                stroke_width: attrs.stroke_width,
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
                style: attrs.style,
            }
        },
        "a" => build_anchor(attrs, children, cfg, navigator),
        _ => {
            rsx! {
                g {
                    id: attrs.id,
                    class: attrs.class,
                    style: attrs.style,
                    "data-unknown-tag": "{tag}",
                    for child in children { {child} }
                }
            }
        }
    };

    Some(el)
}

// ------------------------- Anchor building -------------------------

fn build_anchor(
    a: SvgAttrs,
    children: Vec<Element>,
    cfg: &SvgBuildConfig,
    navigator: Option<&Navigator>,
) -> Element {
    let href = a.href.clone();

    match href {
        Some(h) => match (cfg.classify_link)(&h) {
            LinkKind::External(url) => {
                rsx! {
                    a {
                        id: a.id,
                        class: a.class,
                        href: "{url}",
                        target: a.target.or(Some("_blank".into())),
                        rel: a.rel.or(Some("noopener noreferrer".into())),
                        style: a.style,
                        for child in children { {child} }
                    }
                }
            }
            LinkKind::Internal(mut route) => {
                if let Some(mapper) = cfg.map_internal_route {
                    if let Some(mapped) = mapper(&route) {
                        route = mapped;
                    }
                }
                let route_owned = route.clone();
                // Only intercept if navigator is present
                rsx! {
                    a {
                        id: a.id,
                        class: a.class,
                        href: "{route_owned}",
                        style: a.style,
                        onclick: {
                            let navigator = navigator.cloned();
                                move |evt| {
                                if navigator.is_some() {
                                    evt.prevent_default();
                                    if let Some(nav) = navigator {
                                        nav.push(route_owned.as_str());
                                    }
                                }
                            }
                        },
                        for child in children { {child} }
                    }
                }
            }
            LinkKind::Fragment(id) => {
                let id_owned = id.clone();
                let frag_cb = cfg.on_fragment_click;
                rsx! {
                    a {
                        id: a.id,
                        class: a.class,
                        href: "#{id_owned}",
                        style: a.style,
                        onclick: move |evt| {
                            evt.prevent_default();
                            if let Some(cb) = frag_cb {
                                cb(&id_owned);
                            }
                        },
                        for child in children { {child} }
                    }
                }
            }
            LinkKind::None => {
                rsx! {
                    a {
                        id: a.id,
                        class: a.class,
                        href: h,
                        style: a.style,
                        for child in children { {child} }
                    }
                }
            }
        },
        None => {
            rsx! {
                a {
                    id: a.id,
                    class: a.class,
                    style: a.style,
                    for child in children { {child} }
                }
            }
        }
    }
}

// ------------------------- Utility -------------------------

fn attribute_name(attr: roxmltree::Attribute) -> String {
    match (attr.namespace(), attr.name()) {
        (Some(ns), local) => format!("{ns}:{local}"),
        (None, local) => local.to_string(),
    }
}
