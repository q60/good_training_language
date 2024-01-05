use std::fs;
use std::env;
use std::io;
use std::process::{Command, ExitCode, Stdio};
use std::path::Path;

#[path="./диагностика.rs"]
#[macro_use]
mod диагностика;
#[path="./лексика.rs"]
mod лексика;
#[path="./синтаксис.rs"]
mod синтаксис;
#[path="./компилятор.rs"]
mod компилятор;
#[path="./интерпретатор.rs"]
mod интерпретатор;
#[path="./типизация.rs"]
mod типизация;
#[path="./эльф.rs"]
mod эльф;

use лексика::Лексер;
use компилятор::{Программа, Инструкция, ВидИнструкции, ПП};
use интерпретатор::интерпретировать;

type Результат<Тэ> = Result<Тэ, ()>;

fn прочитать_содержимое_файла(путь_к_файлу: &str) -> Результат<Vec<char>> {
    fs::read_to_string(путь_к_файлу)
        .map(|содержимое| содержимое.chars().collect())
        .map_err(|ошибка| {
            match ошибка.kind() {
                io::ErrorKind::NotFound => eprintln!("ОШИБКА: файл «{путь_к_файлу}» не найден"),
                // СДЕЛАТЬ: локализировать те ошибки, которые возможно.
                _ => eprintln!("ОШИБКА: не получилось прочитать файл «{путь_к_файлу}»: {ошибка}"),
            }
        })
}

struct Команда {
    имя: &'static str,
    сигнатура: &'static str,
    описание: &'static str,
    запустить: fn(программа: &str, аргы: env::Args) -> Результат<()>,
}

const КОМАНДЫ: &[Команда] = &[
    Команда {
        имя: "комп",
        сигнатура: "[-пуск] <путь_к_файлу>",
        описание: "Скомпилировать файлы исходного кода в исполняемый файл для платформы Linux x86_64",
        запустить: |программа, mut аргы| {
            let mut запустить_после_компиляции = false;
            let mut путь_к_файлу = None;

            loop {
                match аргы.next() {
                    Some(арг) => match арг.as_str() {
                        "-пуск" => запустить_после_компиляции = true,
                        _ => {
                            if путь_к_файлу.is_some() {
                                пример(программа);
                                eprintln!("ОШИБКА: неизвестный флаг «{арг}»");
                                return Err(())
                            } else {
                                путь_к_файлу = Some(арг)
                            }
                        }
                    }
                    None => break,
                }
            }

            let путь_к_файлу = if let Some(путь_к_файлу) = путь_к_файлу {
                путь_к_файлу
            } else {
                пример(программа);
                eprintln!("ОШИБКА: требуется файл с программой!");
                return Err(());
            };

            let mut программа = Программа::default();
            let содержимое: Vec<char> = прочитать_содержимое_файла(&путь_к_файлу)?;
            let mut лекс = Лексер::новый(&путь_к_файлу, &содержимое);
            программа.скомпилировать_лексемы(&mut лекс)?;
            let процедура_точки_входа = "главная";
            if let Some(процедура) = программа.имена.процедуры.get(процедура_точки_входа) {
                let путь_к_исполняемому = Path::new("./").join(&путь_к_файлу).with_extension("");
                эльф::сгенерировать(&путь_к_исполняемому, &программа.пп, процедура.точка_входа)?;
                if запустить_после_компиляции {
                    println!("ИНФО: запускаем «{путь_к_исполняемому}»", путь_к_исполняемому = путь_к_исполняемому.display());
                    Command::new(&путь_к_исполняемому)
                        .stdout(Stdio::inherit())
                        .spawn()
                        .map_err(|ошибка| {
                            eprintln!("ОШИБКА: не получилось запустить дочерний процесс {путь_к_исполняемому}: {ошибка}",
                                      путь_к_исполняемому = путь_к_исполняемому.display());
                        })?
                        .wait()
                        .map_err(|ошибка| {
                            eprintln!("ОШИБКА: что-то пошло не так пока мы ждали завершения дочернего процесса {путь_к_исполняемому
}: {ошибка}",
                                      путь_к_исполняемому = путь_к_исполняемому.display());
                        })?;
                }
                Ok(())
            } else {
                eprintln!("ОШИБКА: процедура точки входа «{процедура_точки_входа}» не найдена! Пожалуйста определите её!");
                Err(())
            }
        },
    },
    Команда {
        имя: "интер",
        сигнатура: "[-отлад] <путь_к_файлу>",
        описание: "Интерпретировать Промежуточное Представление скомпилированного файла",
        запустить: |программа, mut аргы| {
            let mut режим_отладки = false;
            let mut путь_к_файлу = None;

            loop {
                match аргы.next() {
                    Some(арг) => match арг.as_str() {
                        "-отлад" => режим_отладки = true,
                        _ => {
                            if путь_к_файлу.is_some() {
                                пример(программа);
                                eprintln!("ОШИБКА: неизвестный флаг «{арг}»");
                                return Err(())
                            } else {
                                путь_к_файлу = Some(арг)
                            }
                        }
                    }
                    None => break,
                }
            }

            let путь_к_файлу = if let Some(путь_к_файлу) = путь_к_файлу {
                путь_к_файлу
            } else {
                пример(программа);
                eprintln!("ОШИБКА: требуется файл с программой!");
                return Err(());
            };

            let содержимое: Vec<char> = прочитать_содержимое_файла(&путь_к_файлу)?;
            let mut лекс = Лексер::новый(&путь_к_файлу, &содержимое);
            let mut программа = Программа::default();
            программа.скомпилировать_лексемы(&mut лекс)?;
            let процедура_точки_входа = "главная";
            if let Some(процедура) = программа.имена.процедуры.get(процедура_точки_входа) {
                интерпретировать(&программа.пп, &программа.имена, процедура.точка_входа, режим_отладки)
            } else {
                eprintln!("ОШИБКА: процедура точки входа «{процедура_точки_входа}» не найдена! Пожалуйста определите её!");
                Err(())
            }
        },
    },
    Команда {
        имя: "пп",
        сигнатура: "<путь_к_файлу>",
        описание: "Напечатать Промежуточное Представление скомпилированной программы",
        запустить: |программа, mut аргы| {
            let путь_к_файлу = if let Some(путь_к_файлу) = аргы.next() {
                путь_к_файлу
            } else {
                пример(программа);
                eprintln!("ОШИБКА: требуется файл с программой!");
                return Err(());
            };
            let содержимое: Vec<char> = прочитать_содержимое_файла(&путь_к_файлу)?;
            let mut лекс = Лексер::новый(&путь_к_файлу, &содержимое);
            let mut программа = Программа::default();
            программа.скомпилировать_лексемы(&mut лекс)?;
            let процедура_точки_входа = "главная";
            if let Some(процедура) = программа.имена.процедуры.get(процедура_точки_входа) {
                программа.пп.вывалить(процедура.точка_входа);
                Ok(())
            } else {
                eprintln!("ОШИБКА: процедура точки входа «{процедура_точки_входа}» не найдена! Пожалуйста определите её!");
                Err(())
            }
        },
    },
    Команда {
        имя: "справка",
        сигнатура: "[команда]",
        описание: "Напечатать справку по программе и командам",
        запустить: |программа, mut аргы| {
            if let Some(_имя_команды) = аргы.next() {
                todo!("СДЕЛАТЬ: справка по отдельным командам");
            } else {
                пример(программа);
                Ok(())
            }
        },
    },
    Команда {
        имя: "отлад",
        сигнатура: "<путь_к_файлу>",
        описание: "Отладочная команда. Полезна только для разработчика компилятора.",
        запустить: |программа, mut аргы| {
            let путь_к_файлу = if let Some(путь_к_файлу) = аргы.next() {
                путь_к_файлу
            } else {
                пример(программа);
                eprintln!("ОШИБКА: требуется файл с программой!");
                return Err(());
            };
            let mut пп = ПП::default();
            // СДЕЛАТЬ: может имеет смысл реализовать генерацию
            // бинарников из вот такого вот ассемблера ПП? Будет очень
            // удобно отлаживать генерацию машинного кода, когда
            // должный синтаксис еще не реализован.
            пп.код.push(Инструкция{ вид: ВидИнструкции::ПротолкнутьЦелое(1), лок: здесь!() });
            пп.код.push(Инструкция{ вид: ВидИнструкции::ЛогОтрицание, лок: здесь!() });
            пп.код.push(Инструкция{ вид: ВидИнструкции::Возврат, лок: здесь!() });
            пп.вывалить(0);
            эльф::сгенерировать(&Path::new(&путь_к_файлу).with_extension(""), &пп, 0)
        }
    },
];

fn пример(программа: &str) {
    eprintln!("Пример: {программа} <команда> [аргументы]");
    eprintln!("Команды:");
    let ширина_столбца_имени = КОМАНДЫ.iter().map(|команда| {
        команда.имя.chars().count()
    }).max().unwrap_or(0);
    let ширина_столбца_сигнатуры = КОМАНДЫ.iter().map(|команда| {
        команда.сигнатура.chars().count()
    }).max().unwrap_or(0);
    for Команда{имя, сигнатура, описание, ..} in КОМАНДЫ.iter() {
        // СДЕЛАТЬ: переносить длинные описания на новую строку.
        eprintln!("    {имя:ширина_столбца_имени$} {сигнатура:ширина_столбца_сигнатуры$} - {описание}");
    }
}

fn главная() -> Результат<()> {
    let mut аргы = env::args();
    let программа = аргы.next().expect("программа");

    let имя_команды = if let Some(имя_команды) = аргы.next() {
        имя_команды
    } else {
        пример(&программа);
        eprintln!("ОШИБКА: требуется команда!");
        return Err(());
    };

    if let Some(команда) = КОМАНДЫ.iter().find(|команда| имя_команды == команда.имя) {
        (команда.запустить)(&программа, аргы)
    } else {
        пример(&программа);
        eprintln!("ОШИБКА: неизвестная команда «{имя_команды}»");
        Err(())
    }
}

fn main() -> ExitCode {
    match главная() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
