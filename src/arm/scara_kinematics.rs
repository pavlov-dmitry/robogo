//Расчёт обратной кинематики
use std::f64::consts::PI;

/// Структура SCARA-манипулятора
#[derive(Debug, Clone)]
pub struct ScaraKinematics {
    pub l1: f64, // длина первого плеча (мм)
    pub l2: f64, // длина второго плеча (мм)
}

/// Результат решения обратной кинематики
#[derive(Debug, Clone, PartialEq)]
pub struct IkSolution {
    pub theta1: f64, // угол первого сустава (рад)
    pub theta2: f64, // угол второго сустава (рад)
}

/// Ошибки при решении IK
#[derive(Debug, PartialEq)]
pub enum IkError {
    Unreachable, // точка недостижима даже с учетом защитного порога
}

impl ScaraKinematics {
    /// Создание нового экземпляра с защитными порогами по умолчанию
    pub fn new(l1: f64, l2: f64) -> Self {
        assert!(
            l1 > 0.0 && l2 > 0.0,
            "Длины звеньев должны быть положительными"
        );

        ScaraKinematics { l1, l2 }
    }

    /// Прямая кинематика
    pub fn forward(&self, theta1: f64, theta2: f64) -> (f64, f64) {
        let total_angle = -PI + theta2;
        // println!(
        //     "forward: {} , {}",
        //     rad_to_degree(theta1),
        //     rad_to_degree(total_angle)
        // );
        let x = self.l1 * theta1.cos() + self.l2 * total_angle.cos();
        let y = self.l1 * theta1.sin() + self.l2 * total_angle.sin();
        (x, y)
    }

    /// Обратная кинематика (только локоть вниз!)
    ///
    /// # Аргументы
    /// * `x`, `y` - целевая позиция схвата
    ///
    /// # Возвращает
    /// * `Ok(IkSolution)` - решение с углами в радианах
    /// * `Err(IkError)` - если решение невозможно
    pub fn inverse(&self, x: f64, y: f64) -> Result<IkSolution, IkError> {
        // вычисляем точки пересения двух окружностей.
        let intersects = intersect_circles(
            Circle {
                x: 0.,
                y: 0.,
                r: self.l1,
            },
            Circle {
                x: x,
                y: y,
                r: self.l2,
            },
        );
        if intersects.len() != 2 {
            return Err(IkError::Unreachable);
        }

        let (a1, b1) = calc_angles_for_point(Point { x: x, y: y }, intersects[0]);
        let (a2, b2) = calc_angles_for_point(Point { x: x, y: y }, intersects[1]);
        //в одну из точек рука чисто технически не может вывернуться. отсечём её по финальным углам.
        //теперь вычисляем углы прихода рук в кажду из точек
        let mut solution = if !greater_rad(b1, a1) {
            IkSolution {
                theta1: a1,
                theta2: b1,
            }
        } else {
            IkSolution {
                theta1: a2,
                theta2: b2,
            }
        };

        solution.theta2 += PI;

        Ok(solution)
    }

    // /// Проверка достижимости точки с учетом защитных порогов
    // pub fn is_reachable(&self, x: f64, y: f64) -> bool {
    //     let r = (x * x + y * y).sqrt();
    //     let r_max = self.l1 + self.l2;
    //     let r_min = (self.l1 - self.l2).abs();

    //     let safe_r_max = r_max - self.safety_margin;
    //     let safe_r_min = r_min + self.safety_margin;

    //     r <= safe_r_max && r >= safe_r_min
    // }

    // /// Получение безопасной рабочей зоны
    // pub fn get_workspace(&self) -> (f64, f64) {
    //     let r_max = self.l1 + self.l2;
    //     let r_min = (self.l1 - self.l2).abs();

    //     (r_min + self.safety_margin, r_max - self.safety_margin)
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

// Написал по мой просьбе AI от гугла
pub fn intersect_circles(c1: Circle, c2: Circle) -> Vec<Point> {
    let dx = c2.x - c1.x;
    let dy = c2.y - c1.y;
    let r = (dx * dx + dy * dy).sqrt();

    // Check for no intersection or containment
    if r > c1.r + c2.r || r < (c1.r - c2.r).abs() || (r == 0.0 && c1.r == c2.r) {
        return vec![];
    }

    let a = (c1.r * c1.r - c2.r * c2.r + r * r) / (2.0 * r);
    let h = (c1.r * c1.r - a * a).max(0.0).sqrt();

    // Find P2, the intermediate point on the line segment between centers
    let p2_x = c1.x + a * dx / r;
    let p2_y = c1.y + a * dy / r;

    // Find the offset vector for the intersection points perpendicular to the center line
    let rx = -dy * (h / r);
    let ry = dx * (h / r);

    // If h is 0, the circles are tangent (one intersection point)
    if h == 0.0 {
        return vec![Point { x: p2_x, y: p2_y }];
    }

    vec![
        Point {
            x: p2_x + rx,
            y: p2_y + ry,
        },
        Point {
            x: p2_x - rx,
            y: p2_y - ry,
        },
    ]
}

pub fn calc_angle(from: Point, to: Point) -> f64 {
    let x_len = to.x - from.x;
    let y_len = to.y - from.y;
    y_len.atan2(x_len)
}

fn calc_angles_for_point(dst_pnt: Point, middle_pnt: Point) -> (f64, f64) {
    let first_angle = calc_angle(Point { x: 0., y: 0. }, middle_pnt);
    let second_angle = calc_angle(middle_pnt, dst_pnt);
    (first_angle, second_angle)
}

fn greater_rad(a: f64, b: f64) -> bool {
    let a = if a > 0. { a } else { 2. * PI + a };
    let b = if b > 0. { b } else { 2. * PI + b };
    a > b
}
