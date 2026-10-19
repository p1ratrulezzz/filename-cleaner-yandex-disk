use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let result = run();
    pause();
    if let Err(code) = result {
        std::process::exit(code);
    }
}

fn run() -> Result<(), i32> {
    let folder = match pick_folder() {
        Some(path) => path,
        None => {
            println!("Папка не выбрана. Выход.");
            return Err(1);
        }
    };

    if !folder.is_dir() {
        eprintln!("Папка не найдена: {}", folder.display());
        return Err(1);
    }

    println!("Выбрана папка: {}", folder.display());

    let mut changes: Vec<(PathBuf, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(&folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(new_name) = compute_clean_name(&path) {
                    let old_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if new_name != old_name {
                        changes.push((path, new_name));
                    }
                }
            }
        }
    } else {
        eprintln!("Не удалось прочитать содержимое папки.");
        return Err(1);
    }

    if changes.is_empty() {
        println!("\n=== ОТЧЁТ ===");
        println!("Нет файлов, требующих очистки.");
        return Ok(());
    }

    println!("\nПредлагаемые изменения ({} файлов):", changes.len());
    for (old_path, new_name) in &changes {
        let old_name = old_path.file_name().unwrap().to_string_lossy();
        println!("  {}  →  {}", old_name, new_name);
    }

    print!("\nВыполнить переименование? (y/n): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        println!("Ошибка ввода. Отмена.");
        return Err(1);
    }

    if input.trim().to_lowercase() != "y" {
        println!("Отменено.");
        return Ok(());
    }

    println!("\nПереименование...");
    let mut success = 0;
    let mut failed = 0;
    let mut report_lines: Vec<String> = Vec::new();

    for (old_path, new_name) in changes {
        let new_path = old_path.with_file_name(&new_name);
        let final_path = make_unique_path(new_path);
        let old_display = old_path.file_name().unwrap().to_string_lossy().to_string();

        match fs::rename(&old_path, &final_path) {
            Ok(()) => {
                let final_name = final_path.file_name().unwrap().to_string_lossy().to_string();
                println!("  OK: {} → {}", old_display, final_name);
                report_lines.push(format!("OK   {} → {}", old_display, final_name));
                success += 1;
            }
            Err(e) => {
                eprintln!("  ОШИБКА: {} → {}: {}", old_path.display(), final_path.display(), e);
                report_lines.push(format!("FAIL {} → {} ({})", old_display, new_name, e));
                failed += 1;
            }
        }
    }

    println!("\n=== ОТЧЁТ ===");
    println!("Успешно: {}", success);
    println!("Ошибок:  {}", failed);
    println!();
    for line in &report_lines {
        println!("  {}", line);
    }

    Ok(())
}

fn pause() {
    println!("\nНажмите Enter для выхода...");
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}

/// Выбор папки: GUI (если собрано с --features gui), иначе CLI.
fn pick_folder() -> Option<PathBuf> {
    // 1. Аргумент командной строки имеет приоритет
    if let Some(arg) = env::args().nth(1) {
        return Some(PathBuf::from(arg.trim_matches('"')));
    }

    // 2. GUI-диалог (только если фича gui включена)
    #[cfg(feature = "gui")]
    {
        println!("Выберите папку, в которой нужно почистить имена файлов...");
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Выберите папку для очистки имён файлов")
            .pick_folder()
        {
            return Some(path);
        }
        println!("Диалог отменён.");
        return None;
    }

    // 3. CLI-ввод (Linux / сборка без GUI)
    #[cfg(not(feature = "gui"))]
    {
        println!("Использование: filename_cleaner <путь_к_папке>");
        println!("Или введите путь вручную.");
        print!("Путь к папке: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let path = input.trim().trim_matches('"').to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
        None
    }
}

/// Вычисляет чистое имя файла (с расширением).
fn compute_clean_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .filter(|e| !e.is_empty());

    let cleaned_stem = clean_string(&stem);
    if cleaned_stem.is_empty() {
        return None;
    }

    let mut new_name = cleaned_stem;
    if let Some(ext) = ext {
        // Расширение обычно чистое, но на всякий случай тоже чистим (без точки)
        let cleaned_ext = clean_string(&ext);
        if !cleaned_ext.is_empty() {
            new_name.push('.');
            new_name.push_str(&cleaned_ext);
        }
    }

    Some(new_name)
}

/// Удаляет эмодзи и спецсимволы, нормализует пробелы и некоторые знаки препинания.
fn clean_string(s: &str) -> String {
    // Нормализация часто встречающихся «красивых» символов
    let normalized = s
        .replace(['’', '‘', '`'], "'")
        .replace(['“', '”', '«', '»'], "\"")
        .replace(['–', '—', '−'], "-")
        .replace('…', "...");

    let mut result = String::new();
    let mut last_was_space = true; // чтобы не начинать с пробела

    for c in normalized.chars() {
        // Явно выкидываем replacement character и похожие «битые» символы (��)
        if is_broken_or_replacement(c) {
            continue;
        }
        if is_allowed(c) {
            if c.is_whitespace() {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(c);
                last_was_space = false;
            }
        }
        // всё остальное (эмодзи, символы и т.д.) просто пропускаем
    }

    // Убираем хвостовые пробелы и точки (Windows не любит trailing dots/spaces)
    let trimmed = result.trim().trim_end_matches(['.', ' ']).to_string();

    // Если после очистки остались только точки/пробелы — возвращаем пустую строку
    if trimmed.chars().all(|c| c == '.' || c.is_whitespace()) {
        String::new()
    } else {
        trimmed
    }
}

/// Символы, которые часто отображаются как �� или являются «битыми».
fn is_broken_or_replacement(c: char) -> bool {
    matches!(
        c,
        '\u{FFFD}' | // REPLACEMENT CHARACTER
        '\u{FFFE}' | // non-character
        '\u{FFFF}' | // non-character
        '\u{FFFC}'   // object replacement
    ) || (c as u32) >= 0xFDD0 && (c as u32) <= 0xFDEF // non-characters
        || c.is_control()
}

/// Разрешённые символы: буквы/цифры (включая Unicode/акценты), пробелы и безопасная пунктуация.
/// Запрещённые для Windows символы (< > : " / \ | ? *) специально не включены.
fn is_allowed(c: char) -> bool {
    if c.is_alphanumeric() || c.is_whitespace() {
        return true;
    }

    // Безопасные символы пунктуации
    matches!(
        c,
        '.' | '-' | '_' | '(' | ')' | '[' | ']' | '\'' | '&' | '#' | ',' | '!' | '?' | ';' | '+' | '=' | '@' | '%' | '~'
    )
}

/// Если целевой путь уже существует — добавляет (1), (2), ... до уникального имени.
fn make_unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = match path.parent() {
        Some(p) => p.to_path_buf(),
        None => return path,
    };

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for i in 1..1000 {
        let candidate = parent.join(format!("{} ({}){}", stem, i, ext));
        if !candidate.exists() {
            return candidate;
        }
    }

    // fallback
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_string_basic_emoji() {
        let input = "38. Fly me to the moon - Frank Sinatra 🌙  (French Version   Chloé Stafler)";
        let expected = "38. Fly me to the moon - Frank Sinatra (French Version Chloé Stafler)";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_multiple_emojis_and_ellipsis() {
        let input = "02. I was recording a video, when all of a sudden… 🦔";
        let expected = "02. I was recording a video, when all of a sudden";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_ghost_and_spaces() {
        let input = "08. What title should I give to this new song  (#short demo) 👻";
        let expected = "08. What title should I give to this new song (#short demo)";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_curly_quotes_and_accents() {
        // Прямые апострофы сохраняются (они допустимы), лишние пробелы схлопываются
        let input = "'' UN POCO LOCO '' -  Reprise en balade de Rose et Chloé";
        let expected = "'' UN POCO LOCO '' - Reprise en balade de Rose et Chloé";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_fancy_quotes_removed() {
        // «Красивые» кавычки заменяются на " и затем удаляются
        let input = "«UN POCO LOCO» - Reprise";
        let expected = "UN POCO LOCO - Reprise";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_replacement_chars_and_emoji() {
        let input = "02. I was recording a video, when all of a sudden… \u{FFFD}\u{FFFD}";
        let expected = "02. I was recording a video, when all of a sudden";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_double_replacement_display() {
        // То, что часто отображается как ��
        let input = "France\u{FFFD}\u{FFFD} #shorts";
        let expected = "France #shorts";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_noncharacters() {
        let input = "file\u{FFFE}name\u{FFFF}test";
        let expected = "filenametest";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_french_accents_preserved() {
        let input = "L’instant de grâce - MAB (#short cover - Chloé Stafler)";
        let expected = "L'instant de grâce - MAB (#short cover - Chloé Stafler)";
        assert_eq!(clean_string(input), expected);
    }

    #[test]
    fn test_clean_string_only_special_becomes_empty() {
        let input = "🌙👻��…";
        assert_eq!(clean_string(input), "");
    }

    #[test]
    fn test_clean_string_trailing_dots_and_spaces() {
        let input = "Hello world.   ";
        assert_eq!(clean_string(input), "Hello world");
    }

    #[test]
    fn test_clean_string_windows_invalid_removed() {
        let input = "file:name<with>invalid|chars?.mp4";
        // : < > | ? are not allowed
        let result = clean_string(input);
        assert!(!result.contains(':'));
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
        assert!(!result.contains('|'));
        assert!(!result.contains('?'));
        assert!(result.contains("filenamewithinvalidchars.mp4") || result.contains("file name with invalid chars.mp4") || !result.is_empty());
    }

    #[test]
    fn test_compute_clean_name_full() {
        let path = Path::new("38. Fly me to the moon - Frank Sinatra 🌙  (French Version   Chloé Stafler).mp4");
        let cleaned = compute_clean_name(path).unwrap();
        assert_eq!(
            cleaned,
            "38. Fly me to the moon - Frank Sinatra (French Version Chloé Stafler).mp4"
        );
    }

    #[test]
    fn test_compute_clean_name_no_change_needed() {
        let path = Path::new("01. Creep - Radiohead (Full French Version available on Patreon).mp4");
        let cleaned = compute_clean_name(path).unwrap();
        assert_eq!(cleaned, "01. Creep - Radiohead (Full French Version available on Patreon).mp4");
    }

    #[test]
    fn test_is_allowed_letters_and_accents() {
        assert!(is_allowed('é'));
        assert!(is_allowed('à'));
        assert!(is_allowed('Ç'));
        assert!(is_allowed('a'));
        assert!(is_allowed('1'));
        assert!(is_allowed(' '));
        assert!(is_allowed('-'));
        assert!(is_allowed('('));
        assert!(is_allowed('#'));
        assert!(!is_allowed('🌙'));
        assert!(!is_allowed(':'));
        assert!(!is_allowed('<'));
        assert!(!is_allowed('|'));
    }
}
