use std::io;
use std::io::{Read, Write, BufRead};
use std::convert::TryInto;
use std::mem::size_of;
use super::Результат;
use компилятор::{ПП, Имена, ВидИнструкции, Инструкция};
use типизация::{Тип};

// Разметка памяти
// |    стек    | инициализированные данные | неинициализированные данные |    куча?    |
// ^            ^
// 0            Начало стека и данных. Стек растет в сторону нуля.

const РАЗМЕР_СЛОВА: usize = size_of::<u64>(); // 64 бита
const ОБЪЁМ_СТЕКА: usize = 1_000_000;         // 1 мегабайт (как на винде)
const НАЧАЛО_СТЕКА: usize = ОБЪЁМ_СТЕКА;
const НАЧАЛО_ДАННЫХ: usize = ОБЪЁМ_СТЕКА;

#[derive(Default)]
struct Машина<'ы> {
    индекс_инструкции: usize, // аналог rip
    кадр: usize,              // аналог rbp
    стек: usize,              // аналог rsp
    // В каком-то смысле, эти переменные выше являются регистрами
    // нашей виртуальной машины, не смотря на то, что машина-то
    // стековая.

    память: Vec<u8>,
    инструкции: &'ы [Инструкция],
}

macro_rules! ошибка_времени_исполнения {
    ($машина:expr, $($аргы:tt)*) => {{
        let индекс_инструкции = $машина.индекс_инструкции;
        if let Some(инструкция) = $машина.инструкции.get(индекс_инструкции) {
            let вид_инструкции = &инструкция.вид;
            let ::диагностика::Лок{путь_к_файлу, строка, столбец} = &инструкция.лок;
            eprint!("{путь_к_файлу}:{строка}:{столбец}: {вид_инструкции:?}: ", путь_к_файлу = путь_к_файлу.display());
        }
        eprint!("ОШИБКА ВРЕМЕНИ ИСПОЛНЕНИЯ: {индекс_инструкции}: ");
        eprintln!($($аргы)*);
    }};
}

impl<'ы> Машина<'ы> {
    fn протолкнуть_значение(&mut self, значение: usize) -> Результат<()> {
        if self.стек == 0 {
            ошибка_времени_исполнения!(self, "Переполнение стека");
            return Err(())
        }
        self.стек -= РАЗМЕР_СЛОВА;
        self.память[self.стек..self.стек+РАЗМЕР_СЛОВА].copy_from_slice(&значение.to_le_bytes());
        Ok(())
    }

    fn вытолкнуть_значение(&mut self) -> Результат<usize> {
        if НАЧАЛО_ДАННЫХ <= self.стек  {
            ошибка_времени_исполнения!(self, "Опустошение стека");
            return Err(())
        }
        let значение = usize::from_le_bytes(self.память[self.стек..self.стек+РАЗМЕР_СЛОВА].try_into().unwrap());
        self.стек += РАЗМЕР_СЛОВА;
        Ok(значение)
    }

    fn срез_памяти(&mut self, адрес: usize, размер: usize) -> Результат<&mut [u8]> {
        let макс = self.память.len();
        if let Some(срез) = self.память.get_mut(адрес..адрес+размер) {
            Ok(срез)
        } else {
            ошибка_времени_исполнения!(self, "Попытка получить доступ к некорректнному диапазону памяти [{начало}..{конец}). Разрешенный диапазон [0..{макс})", начало = адрес, конец = адрес+размер);
            Err(())
        }
    }

    fn количество_элементов_стека(&self) -> usize {
        (ОБЪЁМ_СТЕКА - self.стек)/РАЗМЕР_СЛОВА
    }

    fn проверить_арность_аргументов(&self, арность: usize) -> Результат<()> {
        let размер_стека = self.количество_элементов_стека();
        if размер_стека < арность {
            ошибка_времени_исполнения!(self, "Недостаточно аргументов для инструкции. Требуется как минимум {арность}, но всего в стеке аргументов находится {размер_стека}.");
            Err(())
        } else {
            Ok(())
        }
    }

    fn инструкция(&self) -> Результат<&Инструкция> {
        match self.инструкции.get(self.индекс_инструкции) {
            Some(инструкция) => Ok(инструкция),
            None => {
                ошибка_времени_исполнения!(self, "некорректный индекс инструкции");
                Err(())
            }
        }
    }
}

pub fn интерпретировать(пп: &ПП, имена: &Имена, точка_входа: usize, режим_отладки: bool) -> Результат<()> {
    let mut машина = Машина::default();
    машина.инструкции = &пп.код;
    // СДЕЛАТЬ: Ресайз вектора капец какой медленный. Возможно из-за
    // инициализации. Надо что-нибудь с этим сделать.
    машина.память.resize(ОБЪЁМ_СТЕКА, 0);
    машина.память.extend_from_slice(пп.иниц_данные.as_slice());
    машина.память.resize(машина.память.len() + пп.размер_неиниц_данных, 0);
    машина.индекс_инструкции = точка_входа;
    машина.стек = НАЧАЛО_СТЕКА;
    машина.кадр = НАЧАЛО_СТЕКА;

    let mut глубина_вызовов = 0;
    let mut цель_перешагивания: Option<usize> = None;
    loop {
        let индекс_инструкции = машина.индекс_инструкции;
        let инструкция = машина.инструкция()?;

        if режим_отладки {
            if let Some(цель) = цель_перешагивания.clone() {
                if глубина_вызовов <= цель {
                    цель_перешагивания = None;
                }
            }

            if цель_перешагивания.is_none() {
                диагностика!(&инструкция.лок, "ИНСТРУКЦИЯ", "{индекс_инструкции}: {вид_инструкции:?}", вид_инструкции = инструкция.вид);
                if let ([], стек, []) = unsafe { (&машина.память[машина.стек..НАЧАЛО_СТЕКА]).align_to::<u64>() } {
                    eprintln!("стек = {стек:?}");
                } else {
                    eprintln!("стек = НЕВЫРАВНЕН");
                }
                eprintln!("кадр = {кадр}", кадр = машина.кадр);
                eprintln!("переменные");
                for (имя, переменная) in имена.переменные.iter() {
                    let адрес = переменная.адрес + НАЧАЛО_ДАННЫХ;
                    eprintln!("  {имя}: {адрес:#X} = {:?}", &машина.память[адрес..адрес+переменная.тип.размер()], адрес = переменная.адрес + НАЧАЛО_ДАННЫХ);
                }
                loop {
                    let mut команда = String::new();
                    eprint!("> ");
                    io::stdin().lock().read_line(&mut команда).unwrap();
                    let аргы: Vec<&str> = команда.trim().split(' ').filter(|арг| арг.len() > 0).collect();
                    match аргы.as_slice() {
                        ["выход", ..] => {
                            return Ok(());
                        }
                        ["инст", парам @ ..] => match парам {
                            [инст] => match инст.parse::<usize>() {
                                Ok(индекс_инструкции) => if let Some(инструкция) = пп.код.get(индекс_инструкции) {
                                    диагностика!(&инструкция.лок, "ИНСТРУКЦИЯ", "{индекс_инструкции}: {вид_инструкции:?}", вид_инструкции = инструкция.вид);
                                } else {
                                    eprintln!("ОШИБКА: нету инструкции под номером {индекс_инструкции}")
                                },
                                Err(_ошибка) => {
                                    eprintln!("ОШИБКА: индекс инструкции не является корректным целым числом");
                                },
                            },
                            _ => {
                                eprintln!("Пример: инст [индекс_инструкции]");
                                eprintln!("ОШИБКА: требуется индекс инструкции");
                            }
                        }
                        ["перешаг", ..] => {
                            цель_перешагивания = Some(глубина_вызовов);
                            break
                        }
                        [команда, ..] => {
                            eprintln!("ОШИБКА: неизвестная команда «{команда}»");
                        }
                        [] => {
                            break
                        }
                    }
                }
            }
        }

        match &инструкция.вид {
            ВидИнструкции::Ноп => {
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::ПротолкнутьУказатель(указатель) => {
                машина.протолкнуть_значение(указатель + НАЧАЛО_ДАННЫХ)?;
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::ПротолкнутьЦелое(значение)  => {
                машина.протолкнуть_значение(значение)?;
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::Обменять => {
                машина.проверить_арность_аргументов(2)?;
                let первое = машина.вытолкнуть_значение()?;
                let второе = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(первое)?;
                машина.протолкнуть_значение(второе)?;
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::Вытолкнуть(количество) => {
                машина.проверить_арность_аргументов(количество)?;
                машина.стек += РАЗМЕР_СЛОВА*количество;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::СохранитьКадр => {
                машина.протолкнуть_значение(машина.кадр)?;
                машина.кадр = машина.стек;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ВосстановитьКадр => {
                машина.проверить_арность_аргументов(1)?;
                машина.кадр = машина.вытолкнуть_значение()?;
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::ПрочитатьКадр(смещение) => {
                let адрес = машина.кадр as i32 - (смещение as i32 + 1)*(РАЗМЕР_СЛОВА as i32);
                let значение = u64::from_le_bytes(машина.срез_памяти(адрес as usize, РАЗМЕР_СЛОВА)?.try_into().unwrap()) as usize;
                машина.протолкнуть_значение(значение)?;
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::ЗаписатьКадр(смещение) => {
                машина.проверить_арность_аргументов(1)?;
                let значение = машина.вытолкнуть_значение()?;
                let адрес = машина.кадр as i32 - (смещение as i32 + 1)*(РАЗМЕР_СЛОВА as i32);
                машина.срез_памяти(адрес as usize, РАЗМЕР_СЛОВА)?.copy_from_slice(&значение.to_le_bytes());
                машина.индекс_инструкции += 1;
            }
            &ВидИнструкции::ВызватьВнутреннююПроцедуру(адрекс) => {
                глубина_вызовов += 1;
                машина.протолкнуть_значение(индекс_инструкции + 1)?;
                машина.индекс_инструкции = адрекс;
            }
            ВидИнструкции::ВызватьВнешнююПроцедуру{..} => {
                ошибка_времени_исполнения!(&машина, "вынешние вызовы не поддерживаются в режиме интерпретации");
                return Err(())
            }
            ВидИнструкции::Записать8 => {
                машина.проверить_арность_аргументов(2)?;
                let адрес = машина.вытолкнуть_значение()?;
                let значение = (машина.вытолкнуть_значение()? & 0xFF) as u8;
                let тип = Тип::Цел8;
                машина.срез_памяти(адрес, тип.размер())?.copy_from_slice(&значение.to_le_bytes());
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::Записать32 => {
                сделать!(&инструкция.лок, "Интерпретация инструкции Записать32");
                return Err(());
            }
            ВидИнструкции::Записать64 => {
                машина.проверить_арность_аргументов(2)?;
                let адрес = машина.вытолкнуть_значение()?;
                let значение = машина.вытолкнуть_значение()?;
                let тип = Тип::Цел64;
                машина.срез_памяти(адрес, тип.размер())?.copy_from_slice(&значение.to_le_bytes());
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::Прочитать64 => {
                машина.проверить_арность_аргументов(1)?;
                let адрес = машина.вытолкнуть_значение()?;
                let тип = Тип::Цел64;
                let значение: u64 = u64::from_le_bytes(машина.срез_памяти(адрес, тип.размер())?.try_into().unwrap());
                машина.протолкнуть_значение(значение as usize)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелМеньше => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                if левый < правый {
                    машина.протолкнуть_значение(1)?;
                } else {
                    машина.протолкнуть_значение(0)?;
                }
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелБольше => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                if левый > правый {
                    машина.протолкнуть_значение(1)?;
                } else {
                    машина.протолкнуть_значение(0)?;
                }
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелРавно => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                if левый == правый {
                    машина.протолкнуть_значение(1)?;
                } else {
                    машина.протолкнуть_значение(0)?;
                }
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::КонвертЦел64Вещ32 => {
                сделать!(&инструкция.лок, "Интерпретация инструкции КонвертЦел64Вещ32");
                return Err(());
            }
            ВидИнструкции::ЦелСложение => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(левый + правый)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелВычитание => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(левый - правый)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелУмножение => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(левый * правый)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелДеление => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(левый / правый)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЦелОстаток => {
                машина.проверить_арность_аргументов(2)?;
                let правый = машина.вытолкнуть_значение()?;
                let левый = машина.вытолкнуть_значение()?;
                машина.протолкнуть_значение(левый % правый)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::ЛогОтрицание => {
                машина.проверить_арность_аргументов(1)?;
                let значение = машина.вытолкнуть_значение()?;
                if значение == 0 {
                    машина.протолкнуть_значение(1)?;
                } else {
                    машина.протолкнуть_значение(0)?;
                }
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::Прыжок(индекс) => {
                машина.индекс_инструкции = *индекс;
            }
            &ВидИнструкции::УсловныйПрыжок(индекс) => {
                машина.проверить_арность_аргументов(1)?;
                let значение = машина.вытолкнуть_значение()?;
                if значение == 0 {
                    машина.индекс_инструкции += 1;
                } else {
                    машина.индекс_инструкции = индекс;
                }
            }
            ВидИнструкции::ПечатьСтроки => {
                машина.проверить_арность_аргументов(2)?;
                let указатель = машина.вытолкнуть_значение()?;
                let длинна = машина.вытолкнуть_значение()?;
                let _ = io::stdout().write(машина.срез_памяти(указатель, длинна)?);
                let _ = io::stdout().flush();
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::Ввод => {
                машина.проверить_арность_аргументов(2)?;
                let длинна = машина.вытолкнуть_значение()?;
                let указатель = машина.вытолкнуть_значение()?;
                let размер = io::stdin().read(машина.срез_памяти(указатель, длинна)?).unwrap();
                машина.протолкнуть_значение(размер)?;
                машина.индекс_инструкции += 1;
            }
            ВидИнструкции::Возврат => {
                // СДЕЛАТЬ: Ввести отдельную инструкцию останова.
                // И генерировать точку входа наподобии того, как мы это делаем в эльф.
                // Т.е. точка входа 0. Он прыгает в главную, и после вызывает останов.
                if машина.количество_элементов_стека() == 0 {
                    break;
                }
                машина.индекс_инструкции = машина.вытолкнуть_значение()?;
                глубина_вызовов -= 1;
            },
            ВидИнструкции::СисВызов {..} => {
                ошибка_времени_исполнения!(&машина, "системные вызовы не поддерживаются в режиме интерпретации");
                return Err(())
            }
        }
    }
    Ok(())
}
