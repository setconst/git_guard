use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// A map of known "signatures" to their ideal .gitignore entry.
// If the key file/folder is found, the value should be in .gitignore.
fn get_signatures() -> HashMap<&'static str, &'static str> {
    let mut sigs = HashMap::new();
    // Node / JS
    sigs.insert("node_modules", "node_modules/");
    // Python
    sigs.insert("venv", "venv/");
    sigs.insert(".venv", ".venv/");
    sigs.insert("__pycache__", "__pycache__/");
    // Environment variables / Secrets
    sigs.insert(".env", ".env");
    sigs.insert(".env.local", ".env.local");
    // IDEs
    sigs.insert(".idea", ".idea/");
    sigs.insert(".vscode", ".vscode/");
    // OS specific
    sigs.insert(".DS_Store", ".DS_Store");
    sigs.insert("Thumbs.db", "Thumbs.db");
    // Rust build artifacts (just in case!)
    sigs.insert("target", "target/");
    
    sigs
}

fn main() -> io::Result<()> {
    let project_root = Path::new(".");
    let gitignore_path = project_root.join(".gitignore");

    println!("🛡️  Running Git-Guard scanning...");

    // 1. Read or initialize the current .gitignore contents
    let gitignore_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // 2. Scan the directory for known unignored folders/files
    let signatures = get_signatures();
    let mut detected_rules_to_add = Vec::new();

    // Walk the directory, skipping hidden directories like `.git` to stay fast
    let walker = WalkDir::new(project_root)
        .min_depth(1)
        .max_depth(3) // Don't go too deep to keep it blazing fast
        .into_iter()
        .filter_entry(|e| {
            // Skip the .git directory entirely
            if e.file_name() == ".git" {
                return false;
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let file_name = entry.file_name().to_string_lossy();
        
        if let Some(&rule_to_add) = signatures.get(file_name.as_ref()) {
            // Check if this rule (or a variant of it) is already in the .gitignore
            if !gitignore_content.contains(rule_to_add) {
                // Double check we haven't already flagged it in this run
                if !detected_rules_to_add.contains(&rule_to_add) {
                    detected_rules_to_add.push(rule_to_add);
                }
            }
        }
    }

    // 3. Handle the results
    if detected_rules_to_add.is_empty() {
        println!("✨ Your .gitignore looks well-guarded! No missing rules detected.");
        return Ok(());
    }

    println!("\n⚠️  Found unignored files/folders that should likely be hidden:");
    for rule in &detected_rules_to_add {
        println!("   - {}", rule);
    }

    // 4. Safely append missing rules to .gitignore
    print!("\nWould you like to append these to your .gitignore? (y/N): ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    if response.trim().to_lowercase() == "y" {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore_path)?;

        // Add a newline block header if the file isn't empty
        if !gitignore_content.is_empty() && !gitignore_content.ends_with('\n') {
            writeln!(file)?;
        }
        
        writeln!(file, "\n# Added automatically by Git-Guard")?;
        for rule in detected_rules_to_add {
            writeln!(file, "{}", rule)?;
        }
        
        println!("✅ .gitignore updated successfully!");
    } else {
        println!("❌ Operation cancelled. Watch out before you `git add .`!");
    }

    Ok(())
}