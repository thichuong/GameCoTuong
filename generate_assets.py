from PIL import Image, ImageDraw, ImageFont
import os

def create_piece_image(filename, text, text_color, bg_color):
    size = (128, 128)
    img = Image.new('RGBA', size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Draw circle background
    draw.ellipse([4, 4, 124, 124], fill=bg_color, outline=text_color, width=4)
    
    # Draw text
    # Load default font (or try to find a better one)
    try:
        # Try common Linux paths for DejaVuSans
        font_paths = [
            "DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
            "/usr/share/fonts/google-noto/NotoSans-Bold.ttf",
            "/usr/share/fonts/google-noto/NotoSans-Regular.ttf"
        ]
        font = None
        for path in font_paths:
            try:
                font = ImageFont.truetype(path, 40)
                print(f"Loaded font: {path}")
                break
            except OSError:
                continue
        
        if font is None:
            raise Exception("No suitable font found")
            
    except Exception as e:
        print(f"Font loading failed: {e}. Using default.")
        font = ImageFont.load_default()
        
    # Calculate text position (approximate centering)
    # PIL's default font doesn't support getsize well in newer versions, using bbox
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    
    x = (size[0] - text_width) / 2
    y = (size[1] - text_height) / 2
    
    draw.text((x, y), text, font=font, fill=text_color)
    
    img.save(filename)
    print(f"Generated {filename}")

os.makedirs("assets/textures", exist_ok=True)

pieces = {
    "General": "Gen",
    "Advisor": "Adv",
    "Elephant": "Ele",
    "Horse": "Hor",
    "Chariot": "Cha",
    "Cannon": "Can",
    "Soldier": "Sol"
}

colors = {
    "red": {"text": "#8B0000", "bg": "#F0D9B5"},  # Dark Red text on Wood
    "black": {"text": "#000000", "bg": "#F0D9B5"} # Black text on Wood
}

for color_name, color_vals in colors.items():
    for piece_name, symbol in pieces.items():
        filename = f"assets/textures/{color_name}_{piece_name.lower()}.png"
        create_piece_image(filename, symbol, color_vals["text"], color_vals["bg"])

def create_board_image():
    # Wood color
    color = (222, 184, 135, 255) # Burlywood
    img = Image.new('RGBA', (1024, 1024), color)
    img.save("assets/textures/board.png")
    print("Generated assets/textures/board.png")

create_board_image()
