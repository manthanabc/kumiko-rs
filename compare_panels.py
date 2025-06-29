import json

def normalize_panels(panels_data):
    normalized = []
    if isinstance(panels_data, list) and panels_data and isinstance(panels_data[0], dict):
        # Already in the desired format from Rust
        normalized = panels_data
    elif isinstance(panels_data, list) and panels_data and isinstance(panels_data[0], list):
        # From Python, convert [x, y, w, h] to {"x": x, "y": y, "w": w, "h": h}
        for p in panels_data:
            normalized.append({"x": p[0], "y": p[1], "w": p[2], "h": p[3]})
    return normalized

def compare_json_outputs(rust_json_path, python_json_path):
    with open(rust_json_path, 'r') as f:
        rust_data = json.load(f)

    with open(python_json_path, 'r') as f:
        python_full_data = json.load(f)
        # Extract the 'panels' array from the Python output
        python_panels_data = python_full_data[0]['panels']

    rust_panels = normalize_panels(rust_data)
    python_panels = normalize_panels(python_panels_data)

    print("--- Rust Panels ---")
    for p in rust_panels:
        print(p)
    print("\n--- Python Panels ---")
    for p in python_panels:
        print(p)

    # Simple comparison: check if the number of panels is the same
    if len(rust_panels) != len(python_panels):
        print(f"\nDifference: Number of panels mismatch. Rust: {len(rust_panels)}, Python: {len(python_panels)}")
        return

    # Detailed comparison: check each panel
    differences_found = False
    for i in range(len(rust_panels)):
        r_panel = rust_panels[i]
        p_panel = python_panels[i]
        if r_panel != p_panel:
            print(f"\nDifference in panel {i}:")
            print(f"  Rust: {r_panel}")
            print(f"  Python: {p_panel}")
            differences_found = True
    
    if not differences_found:
        print("\nNo significant differences found in panel coordinates.")
    else:
        print("\nDifferences found in panel coordinates.")

if __name__ == "__main__":
    rust_json = "/mnt/b/Manga_to_Anime/kumiko/kumiko_rs/panels.json"
    python_json = "/mnt/b/Manga_to_Anime/kumiko/kumiko_rs/panels_python.json"
    compare_json_outputs(rust_json, python_json)
