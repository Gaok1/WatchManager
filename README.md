# WatchManager

This repository provides a terminal-based inventory management application for clocks. It leverages Rust and the ratatui crate to create an interactive, text-based user interface, crossterm for terminal handling, and serde + serde_json for data persistence.

## Clock Inventory Management System

This repository provides a terminal-based inventory management application for clocks.  
Built with Rust, it uses:

- **ratatui**: For a rich, interactive, text-based user interface
- **crossterm**: For handling terminal I/O and screen manipulation
- **serde + serde_json**: For data serialization and persistence

The application allows you to:

- **List Inventory:** View all registered clocks and their quantities.
- **Register Clocks:** Add new clocks by specifying a code and initial quantity.
- **Search:** Find clocks by code, including approximate matches.
- **Buy & Sell:** Perform operations to add or remove quantities from the inventory.
- **View History:** Check an operational log (purchases, sales, and acquisitions).
- **Filter History by Code:** Easily filter the historical log for a specific clock code.
- **Bar Chart Visualization:** See a simple bar graph of sales and purchases from the last 7 days.

## Installation & Running

1. Ensure [Rust is installed](https://www.rust-lang.org/tools/install).
2. Clone the repository:
   ```bash
   git clone https://github.com/your-username/clock-inventory.git
   cd clock-inventory
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Run the application:
   ```bash
   cargo run --release
   ```

## Controls

- `C` - Enter Registration mode (to add new clocks).
- `B` - Enter Search mode.
- `H` - Enter History mode (navigate with arrow keys, filter tabs with left/right).
- `G` - Display the Bar Chart mode.
- `A` - Buy/Add inventory for the selected clock.
- `V` - Sell from the selected clock.
- `Enter` - Select item in lists.
- `Esc` - Return to Inventory mode, or cancel the current action.
- `X` - Exit the application.

### In History Mode:

- `←/→` - Switch tabs: All, Purchases, Sales, Acquisitions.
- `P` - Search history by code (press Enter to apply the filter).
- `Up/Down` - Navigate within history results.

## Data Persistence

The application uses serde_json to read and write data to `estoque.json`. Each inventory change (registration, purchase, sale) updates this file, ensuring that data is retained between sessions.

## ScreenShots
### stock Screen
![image](https://github.com/user-attachments/assets/78fe7683-00e4-4d6f-8153-6a820f0b0d8c)

### Buying/selling analytics
![image](https://github.com/user-attachments/assets/4dd135a1-8922-4984-9a7f-6c5c0a8907b8)

### History screen
![image](https://github.com/user-attachments/assets/c040e484-7d38-4ac8-b873-2c58b995db37)

### Searching Screen
![image](https://github.com/user-attachments/assets/939306fa-5607-43c2-875b-c1305f58a3c3)




