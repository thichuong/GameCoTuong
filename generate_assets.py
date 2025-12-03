from PIL import Image, ImageDraw, ImageFont
import os

def create_piece_image(filename, text, text_color, bg_color, font):
    # Supersampling: Draw at 4x resolution and downscale
    target_size = 128
    scale_factor = 4
    size = (target_size * scale_factor, target_size * scale_factor)
    
    img = Image.new('RGBA', size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Draw circle background
    border_width = 4 * scale_factor
    draw.ellipse([border_width, border_width, size[0] - border_width, size[1] - border_width], 
                 fill=bg_color, outline=text_color, width=border_width)
    
    # Calculate text position (centering)
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    
    x = (size[0] - text_width) / 2
    y = (size[1] - text_height) / 2
    
    # Offset y slightly to center visually (fonts often have baseline issues)
    y -= text_height * 0.1 

    draw.text((x, y), text, font=font, fill=text_color)
    
    # Downscale using LANCZOS for high quality antialiasing
    img = img.resize((target_size, target_size), resample=Image.Resampling.LANCZOS)
    
    img.save(filename)
    print(f"Generated {filename}")

os.makedirs("assets/textures", exist_ok=True)

# Xiangqi pieces: (Red Character, Black Character)
pieces_map = {
    "General": ("帥", "將"),
    "Advisor": ("仕", "士"),
    "Elephant": ("相", "象"),
    "Horse": ("傌", "馬"),
    "Chariot": ("俥", "車"),
    "Cannon": ("炮", "砲"),
    "Soldier": ("兵", "卒")
}

colors = {
    "red": {"text": "#8B0000", "bg": "#F0D9B5"},  # Dark Red text on Wood
    "black": {"text": "#000000", "bg": "#F0D9B5"} # Black text on Wood
}

# Load font once
try:
    # Try to find a font that supports Chinese characters
    font_paths = [
        "/usr/share/fonts/google-droid-sans-fonts/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "DroidSansFallbackFull.ttf" # Local fallback
    ]
    
    font_path = None
    for path in font_paths:
        if os.path.exists(path):
            font_path = path
            break
            
    if font_path:
        print(f"Using font: {font_path}")
        # Scale font size by scale_factor (4x)
        # Original size was 40 for 128px, so 160 for 512px
        # Increased to 280 for better visibility
        font = ImageFont.truetype(font_path, 280)
    else:
        print("Warning: No suitable CJK font found. Using default. Characters may not render.")
        font = ImageFont.load_default()
        
except Exception as e:
    print(f"Font loading failed: {e}. Using default.")
    font = ImageFont.load_default()

for piece_name, (red_char, black_char) in pieces_map.items():
    # Generate Red Piece
    filename_red = f"assets/textures/red_{piece_name.lower()}.png"
    create_piece_image(filename_red, red_char, colors["red"]["text"], colors["red"]["bg"], font)
    
    # Generate Black Piece
    filename_black = f"assets/textures/black_{piece_name.lower()}.png"
    create_piece_image(filename_black, black_char, colors["black"]["text"], colors["black"]["bg"], font)

def create_board_image():
    # Wood color
    color = (222, 184, 135, 255) # Burlywood
    img = Image.new('RGBA', (1024, 1024), color)
    img.save("assets/textures/board.png")
    print("Generated assets/textures/board.png")

    create_board_image()
    
    # Create 1x1 white pixel for drawing lines
    pixel = Image.new('RGBA', (1, 1), (255, 255, 255, 255))
    pixel.save("assets/textures/pixel.png")
    print("Generated assets/textures/pixel.png")
