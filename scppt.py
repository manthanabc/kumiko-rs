import json
import sys
import os
import webbrowser
from PIL import Image, ImageDraw, ImageFont

def draw_panel_numbers(json_path, image_path, output_path):
    # Load the image
    try:
        image = Image.open(image_path).convert("RGBA")
    except FileNotFoundError:
        print(f"Error: Image file not found at {image_path}. Skipping.")
        return None
    except Exception as e:
        print(f"Error loading image {image_path}: {e}. Skipping.")
        return None

    draw = ImageDraw.Draw(image)

    # Load a good font with fallback
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", size=24)
    except IOError:
        print("Custom font not found. Using default font.")
        font = ImageFont.load_default()

    # Parse the JSON
    try:
        with open(json_path, 'r') as f:
            panels = json.load(f)
    except FileNotFoundError:
        print(f"Error: JSON file not found at {json_path}. Skipping.")
        return None
    except json.JSONDecodeError as e:
        print(f"Error decoding JSON from {json_path}: {e}. Skipping.")
        return None

    for i, panel in enumerate(panels, 1):
        x, y, w, h = panel['x'], panel['y'], panel['width'], panel['height']
        text = str(i)

        # Ensure width and height are non-negative
        w = max(0, w)
        h = max(0, h)

        # Use getbbox to measure text size
        bbox = font.getbbox(text)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]
        padding = 4

        # Draw semi-transparent black rectangle behind text
        box_width = text_width + 2 * padding
        box_height = text_height + 2 * padding
        overlay = Image.new("RGBA", (box_width, box_height), (0, 0, 0, 200))
        image.paste(overlay, (x, y), overlay)

        # Draw a red rectangle around the panel
        draw.rectangle([x, y, x + w, y + h], outline="red", width=3)

        # Draw text
        draw.text((x + padding, y + padding), text, fill="white", font=font)

    # Save final image
    try:
        image.convert("RGB").save(output_path)
        print(f"✅ Annotated image saved as {output_path}")
        return output_path
    except Exception as e:
        print(f"Error saving image to {output_path}: {e}. Skipping.")
        return None

def open_in_browser(file_path):
    try:
        webbrowser.open(file_path)
        print(f"Opening {file_path} in default web browser.")
    except Exception as e:
        print(f"Error opening browser: {e}")

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python scppt.py <path_to_json_directory> <path_to_image_directory> <path_to_output_directory>")
        sys.exit(1)

    json_dir = sys.argv[1]
    image_dir = os.path.expanduser(sys.argv[2]) # Expand ~ to home directory
    output_dir = sys.argv[3]

    os.makedirs(output_dir, exist_ok=True)

    processed_files = []
    for filename in os.listdir(json_dir):
        if filename.endswith(".json"):
            json_file_path = os.path.join(json_dir, filename)
            base_name = os.path.splitext(filename)[0]
            image_file_path = os.path.join(image_dir, base_name + ".jpg")
            output_file_path = os.path.join(output_dir, "annotated_" + base_name + ".jpg")

            if os.path.exists(image_file_path):
                result_path = draw_panel_numbers(json_file_path, image_file_path, output_file_path)
                if result_path:
                    processed_files.append(result_path)
            else:
                print(f"Warning: Corresponding image not found for {json_file_path} at {image_file_path}. Skipping.")

    if processed_files:
        print(f"Successfully processed {len(processed_files)} files.")
    else:
        print("No files were processed.")
