#include <AccelStepper.h>
#include <Servo.h>
#include <limits.h>

// инкриментируется при изменении уже существующих комманд
const uint16_t VERSION = 2;

const uint16_t MAX_SPEED_INTERVAL = 20000;
const uint16_t MIN_SPEED_INTERVAL = 1000;
const uint16_t ACCELERATION = 500;

AccelStepper motor_x(AccelStepper::DRIVER, 2, 5);
AccelStepper motor_y(AccelStepper::DRIVER, 3, 6);
AccelStepper motor_z(AccelStepper::DRIVER, 4, 7);

const int hallSensorPinX = 9;   // Пин X+ на шилде
const int hallSensorPinY = 10;   // Пин Y+ на шилде
const int hallSensorPinZ = 11;   // Пин Z+ на шилде

Servo lock_servo;
Servo turn_servo;

const int SERVO_TURN_PIN = 13; // Пин SpnDr на шилде
const int SERVO_LOCK_PIN = 12; // Пин SpnEn на шилде

const int ZERO_SPEED = 200;

const auto INVALID_VALUE = INT_MAX;

enum Mode
{
  Mode_Idle, // режим ожидания
  Mode_Move, // режим передвижения
  Mode_Zero // режим выхода в ноль
};

Mode g_currentMode = Mode_Idle;

struct MotorCmd
{
  int steps = INVALID_VALUE;
  int start_speed = INVALID_VALUE;
  int max_speed = INVALID_VALUE;
  int acceleration = INVALID_VALUE;
};

struct MoveCmd
{
  MotorCmd x;
  MotorCmd y;
  MotorCmd z;
};

const size_t MOTORS_COUNT = 3;
const uint8_t ENABLE_PIN = 8;

const int X_MOTOR_IDX = 0; 
const int Y_MOTOR_IDX = 1;
const int Z_MOTOR_IDX = 2;

//предобьявления для компилятора
void parseMoveSubCmd(String subcmd, MoveCmd &moveCmd);
void parseMotorSubCmd(String subcmd, MotorCmd &motorCmd);
void applyCmdTo(AccelStepper &motor, const MotorCmd& cmd);

void setup() {
  Serial.begin(115200);

  turn_servo.attach(SERVO_TURN_PIN);
  lock_servo.attach(SERVO_LOCK_PIN);

  // Включаем внутренний подтягивающий резистор.
  pinMode(hallSensorPinX, INPUT_PULLUP);
  pinMode(hallSensorPinY, INPUT_PULLUP);
  pinMode(hallSensorPinZ, INPUT_PULLUP);


  pinMode(ENABLE_PIN, OUTPUT);
  digitalWrite(ENABLE_PIN, LOW);
}

void loop() {
  // режим перемещения в нужную точку
  if (g_currentMode == Mode_Move) {
    bool anyRunning = false;

    anyRunning |= motor_x.run();
    anyRunning |= motor_y.run();
    anyRunning |= motor_z.run();
    if(!anyRunning) {
      doneCmd();
      g_currentMode = Mode_Idle;
    }
  }

  // Режим выхода в 0
  // Моторы в 0 выодят по очереди X, Y, Z
  // когда крутим X надо подкручивать и Y чтобы из-за конструкции
  // он не менял своего относительного положения
  if (g_currentMode == Mode_Zero) {
    int sensorStateX = digitalRead(hallSensorPinX);
    int sensorStateY = digitalRead(hallSensorPinY);
    if (sensorStateX != LOW) {
      if (sensorStateY == LOW) {
        // подкручиваем следом мотор Y чтобы он не менял относительное положение
        motor_y.setSpeed(ZERO_SPEED);
        motor_y.runSpeed();
      }

      motor_x.setSpeed(ZERO_SPEED);
      motor_x.runSpeed();
    }
    else
    {
      // магнитное поле обнаружено на оси Х
      // мотор Х пришел в 0
      int sensorStateY = digitalRead(hallSensorPinY);
      if (sensorStateY != LOW) {
        motor_y.setSpeed(-ZERO_SPEED);
        motor_y.runSpeed();
      }
      else {
        // мотор Y пришел в 0
        int sensorStateZ = digitalRead(hallSensorPinZ);
        if (sensorStateZ != LOW) {
          motor_z.setSpeed(ZERO_SPEED * 10);
          motor_z.runSpeed();
        }
        else {
          // все моторы пришли в 0
          motor_x.setSpeed(0);
          motor_y.setSpeed(0);
          motor_z.setSpeed(0);
          motor_x.move(0);
          motor_y.move(0);
          motor_z.move(0);
          doneCmd();
          g_currentMode = Mode_Idle;
        }
      }
    }
  }

  // режим ожидания комманд
  if (g_currentMode == Mode_Idle) {
    searchAndProcessCommands();
  }
}

void doneCmd()
{
  Serial.println("done");
} 

void searchAndProcessCommands() {
  if(Serial.available() > 0) {
    String command = Serial.readStringUntil('\n');
    command.trim();

    if (command.length() > 0) {
      command.toLowerCase();
      processCommand(command);
    }
  }
}

void processCommand(String command) {
  if (command == "hi") {
    processHi();
  }
  else if (command == "lock") {
    doLock();
    doneCmd();
  }
  else if (command == "unlock") {
    doUnlock();
    doneCmd();
  }
  else if (command.startsWith("turn")) {
    auto degree = parseTurnCmd(command);
    turnHand(degree);
    doneCmd();
  }
  else if (command.startsWith("zero")) {
    g_currentMode = Mode_Zero;
    motor_z.setMaxSpeed(ZERO_SPEED * 10);
    motor_x.setMaxSpeed(ZERO_SPEED * 10);
    motor_y.setMaxSpeed(ZERO_SPEED * 10);
  }
  else {
    auto cmd = parseMoveCmd(command);
    applyCmdTo(motor_x, cmd.x);
    applyCmdTo(motor_y, cmd.y);
    applyCmdTo(motor_z, cmd.z);
    g_currentMode = Mode_Move;;
  }
}

void processHi() {
    Serial.print("version: ");
    Serial.println(VERSION);
    doneCmd();
}

void applyCmdTo(AccelStepper &motor, const MotorCmd& cmd)
{
  if (cmd.start_speed != INVALID_VALUE) {
    motor.setSpeed(cmd.start_speed);
  }
  if (cmd.max_speed != INVALID_VALUE) {
    motor.setMaxSpeed(cmd.max_speed);
  }
  if (cmd.acceleration != INVALID_VALUE) {
    motor.setAcceleration(cmd.acceleration);
  }
  if (cmd.steps != INVALID_VALUE) {
    motor.move(cmd.steps);
  }
}

MoveCmd parseMoveCmd(String cmd) {
  cmd += " ";

  MoveCmd moveCmd;

  int beginIdx = 0;
  int spaceIdx = cmd.indexOf(' ');  

  while(spaceIdx != -1) {
    String subcmd = cmd.substring(beginIdx, spaceIdx);
    subcmd.trim();
    if(subcmd.length() > 0) {
      parseMoveSubCmd(subcmd, moveCmd);
    }
    beginIdx = spaceIdx + 1;
    spaceIdx = cmd.indexOf(' ', beginIdx);
  }

  return moveCmd;
}

void parseMoveSubCmd(String subcmd, MoveCmd &moveCmd) {
  char type = subcmd.charAt(0);
  auto motorSubCmd = subcmd.substring(1);

  if('x' == type) {
    parseMotorSubCmd(motorSubCmd, moveCmd.x);
  }
  else if ('y' == type) {
    parseMotorSubCmd(motorSubCmd, moveCmd.y);
  }
  else if ('z' == type) {
    parseMotorSubCmd(motorSubCmd, moveCmd.z);
  }
}

void parseMotorSubCmd(String subcmd, MotorCmd &motorCmd) {
  if(subcmd.startsWith("ss")) {
    auto value = subcmd.substring(2).toInt();
    motorCmd.start_speed = value;
  }
  else if (subcmd.startsWith("ms")) {
    auto value = subcmd.substring(2).toInt();
    motorCmd.max_speed = value;
  }
  else if (subcmd.startsWith("a")) {
    auto value = subcmd.substring(1).toInt();
    motorCmd.acceleration = value;
  }
  else {
    auto value = subcmd.toInt();
    motorCmd.steps = value;
  }
}

int parseTurnCmd(String cmd) {
  return cmd.substring(4).toInt();
}

void turnHand(int degree) {
  degree = min(degree, 45);
  degree = max(degree, -45);
  if (degree > 0) {
    degree = degree * 1.15;
  }
  moveServo(turn_servo, 90 + degree, 1, 10);
}

void moveServo(Servo &servo, int degree, int step_degree, int step_delay_ms)
{
  int step = abs(step_degree);
  int current_angle = servo.read();
  if (degree < current_angle)
  {
    step *= -1;
  } 
  while (current_angle != degree) {
    current_angle += step;
    if (step < 0) {
      current_angle = max(current_angle, degree);
    }
    else {
      current_angle = min(current_angle, degree);
    }
    servo.write(current_angle);
    delay(step_delay_ms);
  }
}

void doLock() {
  moveServo(lock_servo, 45, 5, 10);
}

void doUnlock() {
  moveServo(lock_servo, 135, 5, 10);
}
