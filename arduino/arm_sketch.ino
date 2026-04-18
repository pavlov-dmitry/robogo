#include <AccelStepper.h>
#include <limits.h>

// инкриментируется при изменении уже существующих комманд
const uint16_t VERSION = 1;

const uint16_t MAX_SPEED_INTERVAL = 20000;
const uint16_t MIN_SPEED_INTERVAL = 1000;
const uint16_t ACCELERATION = 500;

AccelStepper motor_x(AccelStepper::DRIVER, 2, 5);
AccelStepper motor_y(AccelStepper::DRIVER, 3, 6);
AccelStepper motor_z(AccelStepper::DRIVER, 4, 7);

bool g_isMoveNow = false;

const auto INVALID_VALUE = INT_MAX;

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
  Serial.begin(9600);

  pinMode(ENABLE_PIN, OUTPUT);
  digitalWrite(ENABLE_PIN, LOW);
}

void loop() {
  if (g_isMoveNow) {
    bool anyRunning = false;
    anyRunning |= motor_x.run();
    anyRunning |= motor_y.run();
    anyRunning |= motor_z.run();
    if(!anyRunning) {
      Serial.println("done");
      g_isMoveNow = false;
    }
  }
  else {
    searchAndProcessCommands();
  }
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
  if(command == "hi") {
    processHi();
  }
  else {
    auto cmd = parseMoveCmd(command);
    applyCmdTo(motor_x, cmd.x);
    applyCmdTo(motor_y, cmd.y);
    applyCmdTo(motor_z, cmd.z);
    g_isMoveNow = true;
  }
}

void processHi() {
    Serial.print("version: ");
    Serial.println(VERSION);
    Serial.println("done");
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
  }
  else {
    auto value = subcmd.toInt();\
    motorCmd.steps = value;
  }
}