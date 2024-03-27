use std::num::{IntErrorKind};
use лексика::*;
use диагностика::*;
use super::Результат;

#[derive(Clone)]
pub struct Переменная {
    pub имя: Лексема,
    pub тип: Выражение,
}

impl Переменная {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Переменная> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Двоеточие])?;
        let тип = Выражение::разобрать(лекс)?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
        Ok(Переменная{имя, тип})
    }
}

#[derive(Debug, Clone)]
pub enum ВидБинопа {
    Меньше,
    Больше,
    МеньшеРавно,
    БольшеРавно,
    Сложение,
    Вычитание,
    Умножение,
    Деление,
    Остаток,
    Или,
    И,
    Равно,
    НеРавно,
    Как,
    Поле,
    ЛевоеБитовоеСмещение,
    ПравоеБитовоеСмещение,
}

impl ВидБинопа {
    const МАКС_ПРИОРИТЕТ: usize = 7;

    fn приоритет(&self) -> usize {
        use self::ВидБинопа::*;
        match self {
            Или                                                 => Self::МАКС_ПРИОРИТЕТ - 7,
            И                                                   => Self::МАКС_ПРИОРИТЕТ - 6,
            Меньше | Больше | МеньшеРавно | БольшеРавно | Равно | НеРавно => Self::МАКС_ПРИОРИТЕТ - 5,
            Сложение | Вычитание                                => Self::МАКС_ПРИОРИТЕТ - 4,
            Умножение | Деление | Остаток                       => Self::МАКС_ПРИОРИТЕТ - 3,
            // СДЕЛАТЬ: какой приоритет лучше всего для битовых смещений?
            ЛевоеБитовоеСмещение | ПравоеБитовоеСмещение        => Self::МАКС_ПРИОРИТЕТ - 2,
            Как                                                 => Self::МАКС_ПРИОРИТЕТ - 1,
            Поле                                                => Self::МАКС_ПРИОРИТЕТ,
        }
    }

    fn по_виду_лексемы(вид: &ВидЛексемы) -> Option<ВидБинопа> {
        match вид {
            ВидЛексемы::РавноЛи         => Some(ВидБинопа::Равно),
            ВидЛексемы::МеньшеЛи        => Some(ВидБинопа::Меньше),
            ВидЛексемы::БольшеЛи        => Some(ВидБинопа::Больше),
            ВидЛексемы::МеньшеРавноЛи   => Some(ВидБинопа::МеньшеРавно),
            ВидЛексемы::БольшеРавноЛи   => Some(ВидБинопа::БольшеРавно),
            ВидЛексемы::НеРавноЛи       => Some(ВидБинопа::НеРавно),
            ВидЛексемы::Плюс            => Some(ВидБинопа::Сложение),
            ВидЛексемы::Минус           => Some(ВидБинопа::Вычитание),
            ВидЛексемы::Звёздочка       => Some(ВидБинопа::Умножение),
            ВидЛексемы::ПрямаяНаклонная => Some(ВидБинопа::Деление),
            ВидЛексемы::КлючОст         => Some(ВидБинопа::Остаток),
            ВидЛексемы::КлючКак         => Some(ВидБинопа::Как),
            ВидЛексемы::КлючИли         => Some(ВидБинопа::Или),
            ВидЛексемы::КлючИ           => Some(ВидБинопа::И),
            ВидЛексемы::Точка           => Some(ВидБинопа::Поле),
            ВидЛексемы::КлючЛбс         => Some(ВидБинопа::ЛевоеБитовоеСмещение),
            ВидЛексемы::КлючПбс         => Some(ВидБинопа::ПравоеБитовоеСмещение),
            _                           => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Выражение {
    ЦелЧисло(Лексема, i64),
    НатЧисло(Лексема, u64),
    ВещЧисло(Лексема, f32),
    Лог(Лексема, bool),
    Строка(Лексема),
    Идент(Лексема),
    Вызов{вызываемое: Box<Выражение>, аргументы: Vec<Выражение>},
    УнарныйМинус {
        ключ: Лексема,
        выражение: Box<Выражение>,
    },
    Биноп {
        ключ: Лексема,
        вид: ВидБинопа,
        левое: Box<Выражение>,
        правое: Box<Выражение>,
    },
    Отрицание {
        ключ: Лексема,
        выражение: Box<Выражение>,
    }
}

impl Выражение {
    pub fn лок(&self) -> &Лок {
        match self {
            Выражение::ЦелЧисло(лексема, _) |
            Выражение::НатЧисло(лексема, _) |
            Выражение::ВещЧисло(лексема, _) |
            Выражение::Лог(лексема, _) |
            Выражение::Строка(лексема) |
            Выражение::Идент(лексема) => &лексема.лок,
            Выражение::Биноп{ключ, ..} => &ключ.лок,
            Выражение::Вызов{вызываемое, ..} => вызываемое.лок(),
            Выражение::Отрицание{ключ, ..} => &ключ.лок,
            Выражение::УнарныйМинус{ключ, ..} => &ключ.лок,
        }
    }

    fn разобрать_первичное(лекс: &mut Лексер) -> Результат<Выражение> {
        let лексема = лекс.вытащить_лексему_вида(&[
            ВидЛексемы::ЦелЧисло,
            ВидЛексемы::ВещЧисло,
            ВидЛексемы::Идент,
            ВидЛексемы::Строка,
            ВидЛексемы::ОткрытаяСкобка,
            ВидЛексемы::Не,
            ВидЛексемы::КлючИстина,
            ВидЛексемы::КлючЛожь,
            ВидЛексемы::Минус,
        ])?;
        match лексема.вид {
            ВидЛексемы::ЦелЧисло => {
                let число: u64 = match лексема.текст.parse() {
                    Ok(число) => число,
                    Err(ошибка) => match ошибка.kind() {
                        IntErrorKind::PosOverflow => {
                            диагностика!(&лексема.лок, "ОШИБКА", "Слишком большое целое");
                            return Err(());
                        }
                        IntErrorKind::Empty => unreachable!(),
                        IntErrorKind::InvalidDigit => unreachable!(),
                        IntErrorKind::NegOverflow => unreachable!(),
                        IntErrorKind::Zero => unreachable!(),
                        _ => {
                            диагностика!(&лексема.лок, "ОШИБКА", "Некорректное целое число");
                            return Err(());
                        }
                    }
                };
                if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::Идент {
                    if лекс.подсмотреть_лексему()?.текст == "нат" {
                        let _ = лекс.вытащить_лексему()?;
                        return Ok(Выражение::НатЧисло(лексема, число));
                    }
                }
                Ok(Выражение::ЦелЧисло(лексема, число as i64))
            }
            ВидЛексемы::ВещЧисло => {
                match лексема.текст.parse() {
                    Ok(число) => Ok(Выражение::ВещЧисло(лексема, число)),
                    Err(_ошибка) => {
                        диагностика!(&лексема.лок, "ОШИБКА", "Некорректное вещественное число");
                        Err(())
                    }
                }
            }
            ВидЛексемы::Идент => {
                let mut выражение = Выражение::Идент(лексема);
                while лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ОткрытаяСкобка {
                    let _ = лекс.вытащить_лексему().unwrap();
                    let аргументы = разобрать_список_аргументов_вызова(лекс)?;
                    выражение = Выражение::Вызов {
                        вызываемое: Box::new(выражение),
                        аргументы
                    };
                }
                Ok(выражение)
            },
            ВидЛексемы::Строка => Ok(Выражение::Строка(лексема)),
            ВидЛексемы::ОткрытаяСкобка => {
                let выражение = Выражение::разобрать(лекс)?;
                let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ЗакрытаяСкобка])?;
                Ok(выражение)
            }
            ВидЛексемы::Не => {
                let ключ = лексема;
                let выражение = Box::new(Выражение::разобрать(лекс)?);
                Ok(Выражение::Отрицание{ключ, выражение})
            }
            ВидЛексемы::Минус => {
                let ключ = лексема;
                let выражение = Box::new(Выражение::разобрать_первичное(лекс)?);
                Ok(Выражение::УнарныйМинус{ключ, выражение})
            }
            ВидЛексемы::КлючИстина => Ok(Выражение::Лог(лексема, true)),
            ВидЛексемы::КлючЛожь => Ok(Выражение::Лог(лексема, false)),
            _ => unreachable!(),
        }
    }

    fn разобрать_биноп(лекс: &mut Лексер, приоритет: usize) -> Результат<Выражение> {
        if приоритет > ВидБинопа::МАКС_ПРИОРИТЕТ {
            return Выражение::разобрать_первичное(лекс);
        }

        let mut левое = Выражение::разобрать_биноп(лекс, приоритет + 1)?;
        while let Some(вид) = ВидБинопа::по_виду_лексемы(&лекс.подсмотреть_лексему()?.вид) {
            if вид.приоритет() != приоритет {
                break;
            }
            let ключ = лекс.вытащить_лексему().unwrap();
            let правое = Выражение::разобрать_биноп(лекс, приоритет + 1)?;
            левое = Выражение::Биноп {
                вид,
                ключ,
                левое: Box::new(левое),
                правое: Box::new(правое),
            }
        }
        Ok(левое)
    }

    pub fn разобрать(лекс: &mut Лексер) -> Результат<Выражение> {
        Выражение::разобрать_биноп(лекс, 0)
    }
}

#[derive(Debug)]
pub enum Утверждение {
    Присваивание{ключ: Лексема, левое: Выражение, правое: Выражение},
    Выражение{выражение: Выражение},
    Пока{ключ: Лексема, условие: Выражение, тело: Vec<Утверждение>},
    Для{ключ: Лексема, индекс: Лексема, нижняя_граница: Выражение, верхняя_граница: Выражение, тело: Vec<Утверждение>},
    Если{ключ: Лексема, условие: Выражение, тело: Vec<Утверждение>, иначе: Vec<Утверждение>},
    Вернуть{ключ: Лексема, выражение: Option<Выражение>},
    ДекларацияПеременной{ключ: Лексема, имя: Лексема, тип: Выражение, значение: Option<Выражение>},
    ДекларацияКонстанты{ключ: Лексема, имя: Лексема, значение: Выражение},
}

#[derive(Debug)]
pub struct Параметр {
    pub имя: Лексема,
    pub тип: Выражение,
}

#[derive(Debug)]
pub enum ТелоПроцедуры {
    Внутренее { блок: Vec<Утверждение> },
    Внешнее { символ: Лексема },
}

#[derive(Debug)]
pub struct Процедура {
    pub имя: Лексема,
    pub параметры: Vec<Параметр>,
    pub тип_результата: Option<Выражение>,
    pub тело: ТелоПроцедуры,
}

fn разобрать_утверждение(лекс: &mut Лексер) -> Результат<Утверждение> {
    match лекс.подсмотреть_лексему()?.вид {
        ВидЛексемы::КлючЕсли => {
            let ключ = лекс.вытащить_лексему().unwrap();
            let условие = Выражение::разобрать(лекс)?;
            let тело = разобрать_блок_кода(лекс)?;
            let иначе;
            if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючИначе {
                let _ = лекс.вытащить_лексему()?;
                иначе = разобрать_блок_кода(лекс)?;
            } else {
                иначе = vec![]
            }
            Ok(Утверждение::Если{ключ, условие, тело, иначе})
        }
        ВидЛексемы::КлючПока => {
            let ключ = лекс.вытащить_лексему().unwrap();
            let условие = Выражение::разобрать(лекс)?;
            let тело = разобрать_блок_кода(лекс)?;
            Ok(Утверждение::Пока{ключ, условие, тело})
        }
        ВидЛексемы::КлючДля => {
            let ключ = лекс.вытащить_лексему().unwrap();
            let индекс = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Присваивание])?;
            let нижняя_граница = Выражение::разобрать(лекс)?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаТочка])?;
            let верхняя_граница = Выражение::разобрать(лекс)?;
            let тело = разобрать_блок_кода(лекс)?;
            Ok(Утверждение::Для{ключ, индекс, нижняя_граница, верхняя_граница, тело})
        }
        ВидЛексемы::КлючВернуть => {
            let ключ = лекс.вытащить_лексему().unwrap();
            if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ТочкаЗапятая {
                let _ = лекс.вытащить_лексему().unwrap();
                Ok(Утверждение::Вернуть{ключ, выражение: None})
            } else {
                let выражение = Some(Выражение::разобрать(лекс)?);
                let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                Ok(Утверждение::Вернуть{ключ, выражение})
            }
        }
        ВидЛексемы::КлючПер => {
            let ключ = лекс.вытащить_лексему().unwrap();
            let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Двоеточие])?;
            let тип = Выражение::разобрать(лекс)?;
            let значение = match лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая, ВидЛексемы::Равно])?.вид {
                ВидЛексемы::ТочкаЗапятая => None,
                ВидЛексемы::Равно => {
                    let значение = Выражение::разобрать(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                    Some(значение)
                }
                _ => unreachable!()
            };
            Ok(Утверждение::ДекларацияПеременной{ключ, имя, тип, значение})
        }
        ВидЛексемы::КлючКонст => {
            let ключ = лекс.вытащить_лексему().unwrap();
            let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Равно])?;
            let значение = Выражение::разобрать(лекс)?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
            Ok(Утверждение::ДекларацияКонстанты{ключ, имя, значение})
        }
        _ => {
            let левое = Выражение::разобрать(лекс)?;
            let ключ = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::Присваивание,
                ВидЛексемы::ТочкаЗапятая,
            ])?;
            match ключ.вид {
                ВидЛексемы::Присваивание => {
                    let правое = Выражение::разобрать(лекс)?;
                    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
                    Ok(Утверждение::Присваивание {ключ, левое, правое})
                }
                ВидЛексемы::ТочкаЗапятая => Ok(Утверждение::Выражение {выражение: левое}),
                _ => unreachable!(),
            }
        }
    }
}

fn разобрать_блок_кода(лекс: &mut Лексер) -> Результат<Vec<Утверждение>> {
    let mut блок = Vec::new();
    let ключ = лекс.вытащить_лексему_вида(&[ВидЛексемы::КлючНч, ВидЛексемы::КлючТо])?;
    match ключ.вид {
        ВидЛексемы::КлючНч => loop {
            if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючКц {
                let _ = лекс.вытащить_лексему()?;
                break;
            }
            блок.push(разобрать_утверждение(лекс)?);
        }
        ВидЛексемы::КлючТо => блок.push(разобрать_утверждение(лекс)?),
        _ => unreachable!()
    }
    Ok(блок)
}

fn разобрать_список_аргументов_вызова(лекс: &mut Лексер) -> Результат<Vec<Выражение>> {
    let mut аргументы = Vec::new();

    // СДЕЛАТЬ: ввести идиому лекс.вытащить_лексему_если()
    if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ЗакрытаяСкобка {
        let _ = лекс.вытащить_лексему()?;
    } else {
        'разбор_аргументов: loop {
            аргументы.push(Выражение::разобрать(лекс)?);
            let лексема = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::ЗакрытаяСкобка,
                ВидЛексемы::Запятая
            ])?;
            if лексема.вид == ВидЛексемы::ЗакрытаяСкобка {
                break 'разбор_аргументов
            }
        }
    }
    Ok(аргументы)
}

fn разобрать_список_параметров_процедуры(лекс: &mut Лексер) -> Результат<Vec<Параметр>> {
    let mut параметры: Vec<Параметр> = Vec::new();
    let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ОткрытаяСкобка])?;
    if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::ЗакрытаяСкобка {
        let _ = лекс.вытащить_лексему()?;
    } else {
        'разбор_параметров: loop {
            let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
            if let Some(существующий_параметр) = параметры.iter().find(|параметр| параметр.имя.текст == имя.текст) {
                диагностика!(&имя.лок, "ОШИБКА", "переопределение параметра «{имя}»",
                             имя = имя.текст);
                диагностика!(&существующий_параметр.имя.лок, "ИНФО", "параметр с тем же именем определен тут");
                return Err(());
            }
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Двоеточие])?;
            let тип = Выражение::разобрать(лекс)?;
            параметры.push(Параметр {имя, тип});
            let лексема = лекс.вытащить_лексему_вида(&[
                ВидЛексемы::ЗакрытаяСкобка,
                ВидЛексемы::Запятая
            ])?;
            if лексема.вид == ВидЛексемы::ЗакрытаяСкобка {
                break 'разбор_параметров
            }
        }
    }
    Ok(параметры)
}

impl Процедура {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Процедура> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let параметры = разобрать_список_параметров_процедуры(лекс)?;
        let тип_результата = if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::Двоеточие {
            let _ = лекс.вытащить_лексему().unwrap();
            let тип = Выражение::разобрать(лекс)?;
            Some(тип)
        } else {
            None
        };
        let тело = if лекс.подсмотреть_лексему()?.вид == ВидЛексемы::КлючВнешняя {
            let _ = лекс.вытащить_лексему().unwrap();
            let символ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Строка])?;
            let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
            ТелоПроцедуры::Внешнее {символ}
        } else {
            let блок = разобрать_блок_кода(лекс)?;
            ТелоПроцедуры::Внутренее {блок}
        };
        Ok(Процедура{имя, параметры, тело, тип_результата})
    }
}

#[derive(Debug)]
pub struct Константа {
    pub имя: Лексема,
    pub выражение: Выражение,
}

impl Константа {
    pub fn разобрать(лекс: &mut Лексер) -> Результат<Константа> {
        let имя = лекс.вытащить_лексему_вида(&[ВидЛексемы::Идент])?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::Равно])?;
        let выражение = Выражение::разобрать(лекс)?;
        let _ = лекс.вытащить_лексему_вида(&[ВидЛексемы::ТочкаЗапятая])?;
        Ok(Константа{имя, выражение})
    }
}
