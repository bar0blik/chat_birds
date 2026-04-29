#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Person {
    First = 1,
    Second = 2,
    Third = 3,
}

impl Person {
    pub fn get_pronouns(&self, plural: bool) -> &'static str {
        match (self, plural) {
            (Person::First, false) => "I",
            (Person::Second, _) => "you",
            (Person::Third, false) => "it",
            (Person::First, true) => "we",
            (Person::Third, true) => "they",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPerson;

macro_rules! impl_person_from_int {
    ($($t:ty),* $(,)?) => {
        $(
            impl TryFrom<$t> for Person {
                type Error = InvalidPerson;

                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    match value {
                        1 => Ok(Person::First),
                        2 => Ok(Person::Second),
                        3 => Ok(Person::Third),
                        _ => Err(InvalidPerson),
                    }
                }
            }

            impl From<Person> for $t {
                fn from(value: Person) -> Self {
                    value as $t
                }
            }
        )*
    };
}

impl_person_from_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
