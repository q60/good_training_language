use std::io;
use std::io::{Write, BufRead};
use std::convert::TryInto;
use super::Результат;
use компилятор::{ПП, Имена, ВидИнструкции, Инструкция};
use типизация::{Тип};

fn проверить_арность_аргументов(аргументы: &Vec<usize>, арность: usize, индекс_инструкции: usize, инструкция: &Инструкция) -> Результат<()> {
    if аргументы.len() < арность {
        диагностика!(&инструкция.лок, "ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ", "{индекс_инструкции}: {вид_инструкции:?}: недостаточно аргументов для инструкции. Требуется как минимум {арность}, но всего в стеке аргументов находится {действительность}", действительность = аргументы.len(), вид_инструкции = инструкция.вид);
        Err(())
    } else {
        Ok(())
    }
}

fn проверить_адрес(данные: &Vec<u8>, указатель: usize, индекс_инструкции: usize, инструкция: &Инструкция) -> Результат<()> {
    if указатель >= данные.len() {
        диагностика!(&инструкция.лок, "ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ", "{индекс_инструкции}: {вид_инструкции:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: инструкция попыталась получить доступ к некорректному адресу данных {указатель}. Максимально возможный {макс}.", макс = данные.len(), вид_инструкции = инструкция.вид);
        return Err(())
    } else {
        Ok(())
    }
}

pub fn интерпретировать(пп: &ПП, имена: &Имена, точка_входа: usize, режим_отладки: bool) -> Результат<()> {
    let mut индекс_инструкции = точка_входа;
    let mut стек: Vec<usize> = Vec::new();
    let mut память = пп.иниц_данные.clone();
    let mut кадр = 0;
    память.resize(пп.иниц_данные.len() + пп.размер_неиниц_данных, 0);

    loop {
        let инструкция = match пп.код.get(индекс_инструкции) {
            Some(инструкция) => инструкция,
            None => {
                eprintln!("ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: некорректный индекс инструкции {индекс_инструкции}");
                return Err(());
            }
        };

        match &инструкция.вид {
            ВидИнструкции::Ноп => {
                индекс_инструкции += 1;
            }
            &ВидИнструкции::ПротолкнутьЦелое(значение) | &ВидИнструкции::ПротолкнутьУказатель(значение) => {
                // ЗАМЕЧАНИЕ: Т.к. во время интерпретации, адреса
                // данных начинаются с нуля, реализация
                // ПротолкнутьУказатель ни чем не отличается от
                // ПротолкнутьЦелое.
                стек.push(значение);
                индекс_инструкции += 1;
            }
            &ВидИнструкции::Вытолкнуть(количество) => {
                проверить_арность_аргументов(&стек, количество, индекс_инструкции, &инструкция)?;
                for _ in 0..количество {
                    стек.pop().unwrap();
                }
                индекс_инструкции += 1;
            }
            ВидИнструкции::СохранитьКадр => {
                стек.push(кадр);
                кадр = стек.len();
                индекс_инструкции += 1;
            }
            ВидИнструкции::ВосстановитьКадр => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                кадр = стек.pop().unwrap();
                индекс_инструкции += 1;
            }
            &ВидИнструкции::ПрочитатьКадр(смещение) => {
                let индекс = кадр + смещение;
                if let Some(значение) = стек.get(индекс).cloned() {
                    стек.push(значение);
                } else {
                    eprintln!("{индекс_инструкции}: {вид_инструкции:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: инстуркция попыталась получить доступ к элементу стека под номером {индекс}. Размер стека {размер_стека}.", размер_стека = стек.len(), вид_инструкции = инструкция.вид);
                    return Err(());
                }
                индекс_инструкции += 1;
            }
            &ВидИнструкции::ЗаписатьКадр(смещение) => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let значение = стек.pop().unwrap();

                let индекс = кадр + смещение;
                if let Some(ячейка) = стек.get_mut(индекс) {
                    *ячейка = значение;
                } else {
                    eprintln!("{индекс_инструкции}: {вид_инструкции:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: инстуркция попыталась получить доступ к элементу стека под номером {индекс}. Размер стека {размер_стека}.", размер_стека = стек.len(), вид_инструкции = инструкция.вид);
                    return Err(());
                }
                индекс_инструкции += 1;
            }
            &ВидИнструкции::ВызватьПроцедуру(точка_входа) => {
                стек.push(индекс_инструкции + 1);
                индекс_инструкции = точка_входа;
            }
            ВидИнструкции::Записать64 => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                let значение = стек.pop().unwrap();
                проверить_адрес(&память, указатель, индекс_инструкции, &инструкция)?;
                проверить_адрес(&память, указатель+Тип::Цел8.размер() - 1, индекс_инструкции, &инструкция)?;
                память[указатель..указатель+Тип::Цел8.размер()].copy_from_slice(&значение.to_le_bytes());
                индекс_инструкции += 1;
            }
            ВидИнструкции::Прочитать64 => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                проверить_адрес(&память, указатель, индекс_инструкции, &инструкция)?;
                проверить_адрес(&память, указатель+Тип::Цел8.размер() - 1, индекс_инструкции, &инструкция)?;
                стек.push(usize::from_le_bytes(память[указатель..указатель+Тип::Цел8.размер()].try_into().unwrap()));
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелМеньше => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                if левый < правый {
                    стек.push(1)
                } else {
                    стек.push(0)
                }
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелБольше => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                if левый > правый {
                    стек.push(1)
                } else {
                    стек.push(0)
                }
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелРавно => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                if левый == правый {
                    стек.push(1)
                } else {
                    стек.push(0)
                }
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелСложение => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый + правый);
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелВычитание => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый - правый);
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелУмножение => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый * правый);
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелДеление => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый / правый);
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелОстаток => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый % правый);
                индекс_инструкции += 1;
            }
            ВидИнструкции::ЛогОтрицание => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let значение = стек.pop().unwrap();
                if значение == 0 {
                    стек.push(1);
                } else {
                    стек.push(0)
                }
                индекс_инструкции += 1;
            }
            ВидИнструкции::Прыжок(адрес) => {
                индекс_инструкции = *адрес;
            }
            ВидИнструкции::УсловныйПрыжок(адрес) => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let значение = стек.pop().unwrap();
                if значение == 0 {
                    индекс_инструкции += 1;
                } else {
                    индекс_инструкции = *адрес;
                }
            }
            ВидИнструкции::ПечатьСтроки => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                let длинна = стек.pop().unwrap();

                проверить_адрес(&память, указатель, индекс_инструкции, &инструкция)?;
                if длинна > 0 {
                    проверить_адрес(&память, указатель + длинна - 1, индекс_инструкции, &инструкция)?;
                }
                let _ = io::stdout().write(&память[указатель..указатель + длинна]);

                индекс_инструкции += 1;
            }
            ВидИнструкции::Возврат => {
                if let Some(точка_возврата) = стек.pop() {
                    индекс_инструкции = точка_возврата;
                } else {
                    break;
                }
            },
        }

        if режим_отладки {
            диагностика!(&инструкция.лок, "ИНСТРУКЦИЯ", "{индекс_инструкции}: {вид_инструкции:?}", вид_инструкции = инструкция.вид);
            eprintln!("Стек: {стек:?}");
            for (имя, переменная) in имена.переменные.iter() {
                eprintln!("{имя} = {:?}", &память[переменная.адрес..переменная.адрес+переменная.тип.размер()]);
            }
            loop {
                let mut команда = String::new();
                eprint!("> ");
                io::stdin().lock().read_line(&mut команда).unwrap();
                match команда.trim() {
                    "стек" => {
                        eprintln!("Стек: {стек:?}");
                    }
                    "пер" => {
                        for (имя, переменная) in имена.переменные.iter() {
                            eprintln!("{имя} = {:?}", &память[переменная.адрес..переменная.адрес+переменная.тип.размер()]);
                        }
                    }
                    "выход" => {
                        return Ok(());
                    }
                    "" => {
                        break
                    }
                    команда => {
                        eprintln!("ОШИБКА: неизвестная команда «{команда}»");
                    }
                }
            }
        }
    }
    Ok(())
}
