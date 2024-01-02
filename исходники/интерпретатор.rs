use std::io;
use std::io::Write;
use std::convert::TryInto;
use super::Результат;
use компилятор::{ПП, Инструкция};
use типизация::{Тип, ПримТип};

fn проверить_арность_аргументов(аргументы: &Vec<usize>, арность: usize, индекс_инструкции: usize, инструкция: &Инструкция) -> Результат<()> {
    if аргументы.len() < арность {
        eprintln!("{индекс_инструкции}: {инструкция:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: недостаточно аргументов для инструкции. Требуется как минимум {арность}, но всего в стеке аргументов находится {действительность}", действительность = аргументы.len());
        Err(())
    } else {
        Ok(())
    }
}

fn проверить_адрес(данные: &Vec<u8>, указатель: usize, индекс_инструкции: usize, инструкция: &Инструкция) -> Результат<()> {
    if указатель >= данные.len() {
        eprintln!("{индекс_инструкции}: {инструкция:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: инструкция попыталась получить доступ к некорректному адресу данных {указатель}. Максимально возможный {макс}.", макс = данные.len());
        return Err(())
    } else {
        Ok(())
    }
}

pub fn интерпретировать(пп: &ПП, точка_входа: usize) -> Результат<()> {
    let mut индекс_инструкции = точка_входа;
    let mut стек: Vec<usize> = Vec::new();
    let mut данные = пп.иниц_данные.clone();
    let mut кадр = 0;
    данные.resize(пп.иниц_данные.len() + пп.размер_неиниц_данных, 0);

    loop {
        let инструкция = match пп.код.get(индекс_инструкции) {
            Some(инструкция) => инструкция,
            None => {
                eprintln!("ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: некорректный индекс инструкции {индекс_инструкции}");
                return Err(());
            }
        };

        match инструкция {
            Инструкция::Ноп => {
                индекс_инструкции += 1;
            }
            &Инструкция::ПротолкнутьЦелое(значение) | &Инструкция::ПротолкнутьУказатель(значение) => {
                // ЗАМЕЧАНИЕ: Т.к. во время интерпретации, адреса
                // данных начинаются с нуля, реализация
                // ПротолкнутьУказатель ни чем не отличается от
                // ПротолкнутьЦелое.
                стек.push(значение);
                индекс_инструкции += 1;
            }
            Инструкция::Вытолкнуть => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                стек.pop().unwrap();
                индекс_инструкции += 1;
            }
            Инструкция::СохранитьКадр => {
                стек.push(кадр);
                кадр = стек.len();
                индекс_инструкции += 1;
            }
            Инструкция::ВосстановитьКадр => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                кадр = стек.pop().unwrap();
                индекс_инструкции += 1;
            }
            &Инструкция::ПротолкнутьОтКадра(смещение) => {
                let индекс = кадр + смещение;
                if let Some(значение) = стек.get(индекс).cloned() {
                    стек.push(значение);
                } else {
                    eprintln!("{индекс_инструкции}: {инструкция:?}: ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: инстуркция попыталась получить доступ к элементу стека под номером {индекс}. Размер стек {размер_стека}.", размер_стека = стек.len());
                    return Err(());
                }
                индекс_инструкции += 1;
            }
            &Инструкция::ВызватьПроцедуру(точка_входа) => {
                стек.push(индекс_инструкции + 1);
                индекс_инструкции = точка_входа;
            }
            Инструкция::Записать64 => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                let значение = стек.pop().unwrap();
                проверить_адрес(&данные, указатель, индекс_инструкции, &инструкция)?;
                проверить_адрес(&данные, указатель+Тип::ПримТип(ПримТип::Цел8).размер() - 1, индекс_инструкции, &инструкция)?;
                данные[указатель..указатель+Тип::ПримТип(ПримТип::Цел8).размер()].copy_from_slice(&значение.to_le_bytes());
                индекс_инструкции += 1;
            }
            Инструкция::Прочитать64 => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                проверить_адрес(&данные, указатель, индекс_инструкции, &инструкция)?;
                проверить_адрес(&данные, указатель+Тип::ПримТип(ПримТип::Цел8).размер() - 1, индекс_инструкции, &инструкция)?;
                стек.push(usize::from_le_bytes(данные[указатель..указатель+Тип::ПримТип(ПримТип::Цел8).размер()].try_into().unwrap()));
                индекс_инструкции += 1;
            }
            Инструкция::ЦелМеньше => {
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
            Инструкция::ЦелСложение => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let правый = стек.pop().unwrap();
                let левый = стек.pop().unwrap();
                стек.push(левый + правый);
                индекс_инструкции += 1;
            }
            Инструкция::ЛогОтрицание => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let значение = стек.pop().unwrap();
                if значение == 0 {
                    стек.push(1);
                } else {
                    стек.push(0)
                }
                индекс_инструкции += 1;
            }
            Инструкция::Прыжок(адрес) => {
                индекс_инструкции = *адрес;
            }
            Инструкция::УсловныйПрыжок(адрес) => {
                проверить_арность_аргументов(&стек, 1, индекс_инструкции, &инструкция)?;
                let значение = стек.pop().unwrap();
                if значение == 0 {
                    индекс_инструкции += 1;
                } else {
                    индекс_инструкции = *адрес;
                }
            }
            Инструкция::ПечатьСтроки => {
                проверить_арность_аргументов(&стек, 2, индекс_инструкции, &инструкция)?;
                let указатель = стек.pop().unwrap();
                let длинна = стек.pop().unwrap();

                проверить_адрес(&данные, указатель, индекс_инструкции, &инструкция)?;
                if длинна > 0 {
                    проверить_адрес(&данные, указатель + длинна - 1, индекс_инструкции, &инструкция)?;
                }
                let _ = io::stdout().write(&данные[указатель..указатель + длинна]);

                индекс_инструкции += 1;
            }
            Инструкция::Возврат => {
                if let Some(точка_возврата) = стек.pop() {
                    индекс_инструкции = точка_возврата;
                } else {
                    break;
                }
            },
        }
    }
    Ok(())
}
