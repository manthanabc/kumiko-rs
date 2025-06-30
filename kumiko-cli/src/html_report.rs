use serde_json::Value;
use std::collections::HashMap;

// Global counter for page IDs
static mut PAGE_ID: u32 = 0;

fn get_next_page_id() -> u32 {
    unsafe {
        let id = PAGE_ID;
        PAGE_ID += 1;
        id
    }
}

pub fn header(title: &str, reldir: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>

<head>
<title>Kumiko Reader</title>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<script type="text/javascript" src="{reldir}jquery-3.2.1.min.js"></script>
<script type="text/javascript" src="{reldir}reader.js"></script>
<link rel="stylesheet" media="all" href="{reldir}style.css" />
<style type="text/css">
h2, h3 {{ text-align: center; margin-top: 3em; }}
.sidebyside {{ display: flex; justify-content: space-around; }}
.sidebyside > div {{ width: 45%; }}
.version, .step-info {{ text-align: center; }}
.kumiko-reader.halfwidth {{ max-width: 45%; }}
.kumiko-reader.fullpage {{ width: 100%; height: 100%; }}
</style>
</head>

<body>
<h1>{title}</h1>

"#,
        title = title,
        reldir = reldir
    )
}

pub fn nbdiffs(files_diff: &[String]) -> String {
    format!("<p>{} differences found in files</p>", files_diff.len())
}

pub fn side_by_side_panels(
    title: &str,
    step_info: &str,
    jsons: &[Value],
    v1: &str,
    v2: &str,
    images_dir: &str,
    known_panels: &[Vec<Vec<i32>>],
) -> String {
    let mut html = format!(
        r#"
<h2>{title}</h2>
<p class="step-info">{step_info}</p>
<div class="sidebyside">
<div class="version">{v1}</div>
<div class="version">{v2}</div>
</div>
<div class="sidebyside">
"#,
        title = title,
        step_info = step_info,
        v1 = v1,
        v2 = v2
    );

    let oneside_template = r#"
<div id="page{id}" class="kumiko-reader halfwidth debug"></div>
<script type="text/javascript">
var reader = new Reader({{
container: $('#page{id}'),
comicsJson: {json},
images_dir: '{images_dir}',
known_panels: {known_panels}
}});
reader.start();
</script>
"#;

    for (i, js) in jsons.iter().enumerate() {
        let page_id = get_next_page_id();
        let known_panels_json =
            serde_json::to_string(&known_panels[i]).unwrap_or_else(|_| "[]".to_string());
        html += &oneside_template
            .replace("{id}", &page_id.to_string())
            .replace(
                "{json}",
                &serde_json::to_string(js).unwrap_or_else(|_| "{}".to_string()),
            )
            .replace("{images_dir}", images_dir)
            .replace("{known_panels}", &known_panels_json);
    }

    html += "</div>";
    html
}

pub fn imgbox(images: &[HashMap<&str, String>]) -> String {
    let mut html = "<h3>Debugging images</h3>\n<div class=\'imgbox\'>\n".to_string();
    for img in images {
        html += &format!(
            "\t<div><p>{}</p><img src=\"{}\" /></div>\n",
            img["label"], img["filename"]
        );
    }
    html + "</div>\n\n"
}

pub fn reader(js: &Value, images_dir: &str) -> String {
    format!(
        r#"
<div id="reader" class="kumiko-reader fullpage"></div>
<script type="text/javascript">
var reader = new Reader({{
container: $('#reader'),
comicsJson: {json},
images_dir: '{images_dir}',
controls: true
}});
reader.start();
</script>
"#,
        json = serde_json::to_string(js).unwrap_or_else(|_| "{}".to_string()),
        images_dir = images_dir
    )
}

pub fn footer() -> String {
    r#"

</body>
</html>
"#
    .to_string()
}
