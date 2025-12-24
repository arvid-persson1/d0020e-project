use either::Either;

pub use query_macro::Queryable;

mod combinators;
pub use combinators::*;

mod translate;
pub use translate::*;

#[derive(Debug, Clone)]
pub struct Field<T, V: ?Sized, const NAME: &'static str> {
    getter: fn(&T) -> &V,
}

impl<T, V: ?Sized, const NAME: &'static str> Field<T, V, NAME> {
    const fn new(getter: fn(&T) -> &V) -> Self {
        Self { getter }
    }

    pub fn eq<'a>(&self, value: &'a V) -> Eq<'a, fn(&T) -> &V, V> {
        let Self { getter } = self;
        Eq {
            getter: *getter,
            value,
        }
    }

    pub fn ne<'a>(&self, value: &'a V) -> Ne<'a, fn(&T) -> &V, V> {
        let Self { getter } = self;
        Ne {
            getter: *getter,
            value,
        }
    }
}

impl<T, V: PartialOrd + ?Sized, const NAME: &'static str> Field<T, V, NAME> {
    pub fn gt<'a>(&self, value: &'a V) -> Gt<'a, fn(&T) -> &V, V> {
        let Self { getter } = self;
        Gt {
            getter: *getter,
            value,
        }
    }

    pub fn lt<'a>(&self, value: &'a V) -> Lt<'a, fn(&T) -> &V, V> {
        let Self { getter } = self;
        Lt {
            getter: *getter,
            value,
        }
    }
}

/*
pub trait Translatable<C> {
    type Translated;
    type Residual;

    fn translate(
        &self,
        context: &C,
    ) -> Translation<Self::Output, Self::Residual>;
}

// HTTP translator - produces key-value pairs for reqwest
#[derive(Debug, Clone)]
pub struct HttpTranslator;

impl HttpTranslator {
    pub fn new() -> Self {
        Self
    }
}

// Implement HTTP translation for Eq expressions
impl<'a, T, V, const NAME: &'static str> Translatable<HttpTranslator> for Eq<'a, fn(&T) -> &V, V>
where
    V: ToString + ?Sized,
{
    type Output = Vec<(String, String)>;
    type Residual = ();

    fn translate(
        &self,
        _context: &HttpTranslator,
    ) -> Result<Translation<Self::Output, Self::Residual>, TranslationError> {
        Ok(Translation::Full(vec![(
            NAME.to_string(),
            self.1.to_string(),
        )]))
    }
}

// HTTP translation for And - combine all translatable parts
impl<L, R, C> Translatable<C> for And<L, R>
where
    L: Translatable<C>,
    R: Translatable<C>,
    L::Output: IntoIterator<Item = (String, String)>,
    R::Output: IntoIterator<Item = (String, String)>,
{
    type Output = Vec<(String, String)>;
    type Residual = And<L::Residual, R::Residual>;

    fn translate(
        &self,
        context: &C,
    ) -> Result<Translation<Self::Output, Self::Residual>, TranslationError> {
        match (self.0.translate(context), self.1.translate(context)) {
            (Ok(Translation::Full(left)), Ok(Translation::Full(right))) => {
                let mut result: Vec<(String, String)> = left.into_iter().collect();
                result.extend(right);
                Ok(Translation::Full(result))
            },
            (
                Ok(Translation::Full(left)),
                Ok(Translation::Partial {
                    translated: right,
                    residual: r_res,
                }),
            ) => {
                let mut result: Vec<(String, String)> = left.into_iter().collect();
                result.extend(right);
                Ok(Translation::Partial {
                    translated: result,
                    residual: And((), r_res),
                })
            },
            (
                Ok(Translation::Partial {
                    translated: left,
                    residual: l_res,
                }),
                Ok(Translation::Full(right)),
            ) => {
                let mut result: Vec<(String, String)> = left.into_iter().collect();
                result.extend(right);
                Ok(Translation::Partial {
                    translated: result,
                    residual: And(l_res, ()),
                })
            },
            (
                Ok(Translation::Partial {
                    translated: left,
                    residual: l_res,
                }),
                Ok(Translation::Partial {
                    translated: right,
                    residual: r_res,
                }),
            ) => {
                let mut result: Vec<(String, String)> = left.into_iter().collect();
                result.extend(right);
                Ok(Translation::Partial {
                    translated: result,
                    residual: And(l_res, r_res),
                })
            },
            _ => Err(TranslationError::UnsupportedOperation("AND".to_string())),
        }
    }
}

// Or cannot be translated to HTTP params (no OR in query strings)
impl<L, R, C> Translatable<C> for Or<L, R> {
    type Output = Vec<(String, String)>;
    type Residual = Or<L, R>;

    fn translate(
        &self,
        _context: &C,
    ) -> Result<Translation<Self::Output, Self::Residual>, TranslationError> {
        // HTTP query strings don't support OR, so we can't translate this
        Ok(Translation::Partial {
            translated: Vec::new(),
            residual: self.clone(),
        })
    }
}

// Helper for building queries
pub struct QueryBuilder<T, E> {
    expr: E,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<T> QueryBuilder<T, True> {
    pub fn new() -> Self {
        Self {
            expr: True,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T, E> QueryBuilder<T, E>
where
    E: Query<T>,
{
    pub fn and<R>(self, other: R) -> QueryBuilder<T, And<E, R>>
    where
        R: Query<T>,
    {
        QueryBuilder {
            expr: And(self.expr, other),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn or<R>(self, other: R) -> QueryBuilder<T, Or<E, R>>
    where
        R: Query<T>,
    {
        QueryBuilder {
            expr: Or(self.expr, other),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn build(self) -> E {
        self.expr
    }
}
*/

#[cfg(false)]
mod example {
    use query_macro::Queryable;

    #[derive(Queryable)]
    struct Product {
        id: i32,
        name: String,
        price: f64,
        category: String,
    }

    fn main() {
        use super::*;

        // Clean API as requested
        let query = Product::id()
            .eq(&42)
            .and(Product::name().eq("foo"))
            .and(Product::price().lt(&100.0))
            .or(Product::category().eq("electronics"));

        // Test product
        let product = Product {
            id: 42,
            name: "foo".to_string(),
            price: 99.99,
            category: "electronics".to_string(),
        };

        // Local evaluation
        assert!(query.evaluate(&product));

        // HTTP translation
        let translator = HttpTranslator::new();
        match query.translate(&translator) {
            Ok(Translation::Full(params)) => {
                // Use with reqwest
                let client = reqwest::blocking::Client::new();
                let request = client.get("http://example.com/products").query(&params); // params is Vec<(String, String)>
                println!("Full HTTP translation: {:?}", params);
            },
            Ok(Translation::Partial {
                translated: params,
                residual,
            }) => {
                // Partial translation - we can send the translatable part
                // and apply the residual locally
                println!("Partial HTTP translation: {:?}", params);
                println!("Residual to apply locally: {:?}", residual);

                // We could fetch with partial params, then filter results
                let products = vec![product.clone()]; // Simulated fetch
                let filtered: Vec<_> = products
                    .into_iter()
                    .filter(|p| residual.evaluate(p))
                    .collect();
                println!("Filtered to {} products", filtered.len());
            },
            Err(e) => {
                eprintln!("Translation error: {:?}", e);
            },
        }

        // SQL translation example (simplified)
        #[derive(Debug, Clone)]
        struct SqlTranslator;

        impl SqlTranslator {
            fn new() -> Self {
                Self
            }
        }

        // SQL translation for Eq
        impl<'a, T, V, const NAME: &'static str> Translatable<SqlTranslator> for Eq<'a, fn(&T) -> &V, V>
        where
            V: std::fmt::Display + ?Sized,
        {
            type Output = String;
            type Residual = ();

            fn translate(
                &self,
                _context: &SqlTranslator,
            ) -> Result<Translation<Self::Output, Self::Residual>, TranslationError> {
                let value = format!("{}", self.1);
                let escaped_value = if value.contains('\'') {
                    // Simple escaping for demonstration
                    value.replace('\'', "''")
                } else {
                    value
                };

                Ok(Translation::Full(format!("{} = '{}'", NAME, escaped_value)))
            }
        }

        // SQL translation for And
        impl<L, R> Translatable<SqlTranslator> for And<L, R>
        where
            L: Translatable<SqlTranslator, Output = String>,
            R: Translatable<SqlTranslator, Output = String>,
        {
            type Output = String;
            type Residual = And<L::Residual, R::Residual>;

            fn translate(
                &self,
                context: &SqlTranslator,
            ) -> Result<Translation<Self::Output, Self::Residual>, TranslationError> {
                match (self.0.translate(context), self.1.translate(context)) {
                    (Ok(Translation::Full(left)), Ok(Translation::Full(right))) => {
                        Ok(Translation::Full(format!("({}) AND ({})", left, right)))
                    },
                    (
                        Ok(Translation::Full(left)),
                        Ok(Translation::Partial {
                            translated: right,
                            residual: r_res,
                        }),
                    ) => Ok(Translation::Partial {
                        translated: format!("({}) AND ({})", left, right),
                        residual: And((), r_res),
                    }),
                    (
                        Ok(Translation::Partial {
                            translated: left,
                            residual: l_res,
                        }),
                        Ok(Translation::Full(right)),
                    ) => Ok(Translation::Partial {
                        translated: format!("({}) AND ({})", left, right),
                        residual: And(l_res, ()),
                    }),
                    (
                        Ok(Translation::Partial {
                            translated: left,
                            residual: l_res,
                        }),
                        Ok(Translation::Partial {
                            translated: right,
                            residual: r_res,
                        }),
                    ) => Ok(Translation::Partial {
                        translated: format!("({}) AND ({})", left, right),
                        residual: And(l_res, r_res),
                    }),
                    _ => Err(TranslationError::UnsupportedOperation("AND".to_string())),
                }
            }
        }

        // Test SQL translation
        let sql_translator = SqlTranslator::new();
        let simple_query = Product::id().eq(&42).and(Product::name().eq("foo's bar"));

        match simple_query.translate(&sql_translator) {
            Ok(Translation::Full(sql)) => {
                println!("SQL query: WHERE {}", sql);
                // Should produce: WHERE (id = '42') AND (name = 'foo''s bar')
            },
            _ => {},
        }
    }
}
