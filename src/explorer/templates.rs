//! HTML template for the explorer UI.
//!
//! Produces a self-contained single-page application that fetches tool
//! metadata from `./tools` and renders a browsable list with optional
//! execution forms.

/// Render the explorer HTML page with the given configuration values
/// interpolated into the template.
///
/// The returned string is a complete, self-contained HTML document with
/// inline CSS and JS — no external resource loads.
pub fn render_html(
    title: &str,
    project_name: Option<&str>,
    project_url: Option<&str>,
    allow_execute: bool,
) -> String {
    let allow_execute_attr = if allow_execute {
        r#" data-allow-execute="true""#
    } else {
        ""
    };
    let footer = match (project_name, project_url) {
        (Some(name), Some(url)) => {
            format!(r#"<footer><p><a href="{url}">{name}</a></p></footer>"#)
        }
        (Some(name), None) => {
            format!(r#"<footer><p>{name}</p></footer>"#)
        }
        (None, Some(url)) => {
            format!(r#"<footer><p><a href="{url}">{url}</a></p></footer>"#)
        }
        (None, None) => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
*,*::before,*::after{{box-sizing:border-box}}
body{{font-family:system-ui,-apple-system,sans-serif;margin:0;padding:0;background:#f8f9fa;color:#212529}}
header{{background:#212529;color:#fff;padding:1rem 2rem}}
h1{{margin:0;font-size:1.4rem}}
main{{max-width:960px;margin:2rem auto;padding:0 1rem}}
.tool{{background:#fff;border:1px solid #dee2e6;border-radius:6px;padding:1rem 1.25rem;margin-bottom:1rem}}
.tool h2{{margin:0 0 .25rem;font-size:1.1rem}}
.tool p{{margin:0 0 .5rem;color:#495057}}
.schema{{background:#f1f3f5;padding:.75rem;border-radius:4px;font-family:monospace;font-size:.85rem;white-space:pre-wrap;overflow-x:auto}}
.exec-form textarea{{width:100%;min-height:80px;font-family:monospace;font-size:.85rem;margin:.5rem 0}}
.exec-form button{{background:#0d6efd;color:#fff;border:none;padding:.4rem 1rem;border-radius:4px;cursor:pointer}}
.exec-form button:hover{{background:#0b5ed7}}
.result{{background:#e9ecef;padding:.75rem;border-radius:4px;margin-top:.5rem;font-family:monospace;font-size:.85rem;white-space:pre-wrap}}
.error{{color:#dc3545}}
footer{{text-align:center;padding:1rem;color:#6c757d;font-size:.85rem}}
footer a{{color:#0d6efd}}
#loading{{text-align:center;padding:2rem;color:#6c757d}}
</style>
</head>
<body{allow_execute_attr}>
<header><h1>{title}</h1></header>
<main>
<div id="tools"><p id="loading">Loading tools...</p></div>
</main>
{footer}
<script>
(function(){{
  var container=document.getElementById('tools');
  fetch('./tools')
    .then(function(r){{return r.json()}})
    .then(function(tools){{
      container.innerHTML='';
      if(!tools.length){{container.innerHTML='<p>No tools registered.</p>';return}}
      tools.forEach(function(t){{
        var div=document.createElement('div');
        div.className='tool';
        var h2=document.createElement('h2');
        h2.textContent=t.name;
        div.appendChild(h2);
        var p=document.createElement('p');
        p.textContent=t.description;
        div.appendChild(p);
        var schema=document.createElement('pre');
        schema.className='schema';
        schema.textContent=JSON.stringify(t.inputSchema,null,2);
        div.appendChild(schema);
        // Execution form (conditionally shown via data attribute)
        if(document.body.dataset.allowExecute==='true'){{
          var form=document.createElement('div');
          form.className='exec-form';
          var ta=document.createElement('textarea');
          ta.placeholder='{{}}';
          form.appendChild(ta);
          var btn=document.createElement('button');
          btn.textContent='Execute';
          btn.addEventListener('click',function(){{
            var args;
            try{{args=JSON.parse(ta.value||'{{}}')}}catch(e){{alert('Invalid JSON');return}}
            btn.disabled=true;btn.textContent='Running...';
            fetch('./tools/'+encodeURIComponent(t.name)+'/call',{{
              method:'POST',
              headers:{{'Content-Type':'application/json'}},
              body:JSON.stringify(args)
            }})
            .then(function(r){{return r.json()}})
            .then(function(res){{
              var rd=form.querySelector('.result');
              if(!rd){{rd=document.createElement('pre');rd.className='result';form.appendChild(rd)}}
              if(res.is_error){{rd.classList.add('error')}}else{{rd.classList.remove('error')}}
              rd.textContent=JSON.stringify(res,null,2);
            }})
            .catch(function(e){{alert('Error: '+e)}})
            .finally(function(){{btn.disabled=false;btn.textContent='Execute'}});
          }});
          form.appendChild(btn);
          div.appendChild(form);
        }}
        container.appendChild(div);
      }});
    }})
    .catch(function(e){{
      container.innerHTML='<p class="error">Failed to load tools: '+e+'</p>';
    }});
}})();
</script>
</body>
</html>"##,
        title = title,
        footer = footer,
        allow_execute_attr = allow_execute_attr,
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_html_contains_title() {
        let html = render_html("Test Title", None, None, false);
        assert!(
            html.contains("<title>Test Title</title>"),
            "HTML must contain <title> with the configured title"
        );
        assert!(
            html.contains("<h1>Test Title</h1>"),
            "HTML must contain <h1> with the configured title"
        );
    }

    #[test]
    fn render_html_fetches_tools_endpoint() {
        let html = render_html("T", None, None, false);
        assert!(
            html.contains("fetch('./tools')"),
            "HTML JS must fetch the ./tools endpoint"
        );
    }

    #[test]
    fn render_html_with_project_name_and_url() {
        let html = render_html("Title", Some("MyProject"), Some("https://example.com"), false);
        assert!(
            html.contains("MyProject"),
            "Footer must contain the project name"
        );
        assert!(
            html.contains("https://example.com"),
            "Footer must contain the project URL"
        );
        assert!(
            html.contains(r#"<a href="https://example.com">MyProject</a>"#),
            "Project name should be a link to the project URL"
        );
    }

    #[test]
    fn render_html_no_footer_when_no_project() {
        let html = render_html("T", None, None, false);
        assert!(
            !html.contains("<footer>"),
            "No footer when project_name and project_url are both None"
        );
    }

    #[test]
    fn render_html_project_name_only() {
        let html = render_html("T", Some("Proj"), None, false);
        assert!(html.contains("<footer>"));
        assert!(html.contains("Proj"));
        assert!(!html.contains("<a href="));
    }

    #[test]
    fn render_html_is_self_contained() {
        let html = render_html("T", None, None, false);
        // Must not reference external CSS or JS
        assert!(!html.contains("link rel=\"stylesheet\""));
        assert!(!html.contains("<script src="));
    }

    #[test]
    fn render_html_has_execution_form_logic() {
        let html = render_html("T", None, None, false);
        // The JS should check data-allow-execute attribute
        assert!(
            html.contains("allowExecute"),
            "JS must reference allowExecute data attribute for conditional execution"
        );
        // The JS should POST to ./tools/{name}/call
        assert!(
            html.contains("/call"),
            "JS must have the /call endpoint for tool execution"
        );
    }

    #[test]
    fn render_html_valid_doctype() {
        let html = render_html("T", None, None, false);
        assert!(html.starts_with("<!DOCTYPE html>"));
    }
}
