#include <AccelStepper.h>

// инкриментируется при изменении уже существующих комманд
const uint16_t MAJOR_VERSION = 0;
// инкриментируется при добавлении новых
const uint16_t MINOR_VERSION = 1;

const uint16_t MAX_SPEED_INTERVAL = 20000;
const uint16_t MIN_SPEED_INTERVAL = 1000;
const uint16_t ACCELERATION = 500;

AccelStepper motor_x(AccelStepper::DRIVER, 2, 5);
AccelStepper motor_y(AccelStepper::DRIVER, 3, 6);
AccelStepper motor_z(AccelStepper::DRIVER, 4, 7);

bool g_isMoveNow = false;

struct MoveCmd
{
  int x = 0;
  int y = 0;
  int z = 0;
  int s = 0;
  int a = 0;
    
  bool is_empty() {
    return x ==0 && y == 0 && z == 0 && s == 0 && a == 0;
  }
};

const size_t MOTORS_COUNT = 3;
const uint8_t ENABLE_PIN = 8;

const int X_MOTOR_IDX = 0; 
const int Y_MOTOR_IDX = 1;
const int Z_MOTOR_IDX = 2;

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
    if (!cmd.is_empty()) {
      if (cmd.s != 0) {
        //g_cruiseSpeed = constrain(cmd.s, MIN_SPEED_INTERVAL, MAX_SPEED_INTERVAL);
        Serial.print("speed: "); Serial.println(cmd.s);
        motor_x.setMaxSpeed(cmd.s);
        motor_y.setMaxSpeed(cmd.s);
        motor_z.setMaxSpeed(cmd.s);
        
      }
      if (cmd.a != 0) {
        Serial.print("accel: "); Serial.println(cmd.a);
        motor_x.setAcceleration(cmd.a);
        motor_y.setAcceleration(cmd.a);        
        motor_z.setAcceleration(cmd.a);
      }
      if (cmd.x != 0) {
        Serial.print("x: "); Serial.println(cmd.x);
        motor_x.move(cmd.x);
      }
      if (cmd.y != 0) {
        Serial.print("y: "); Serial.println(cmd.y);
        motor_y.move(cmd.y);

      }
      if (cmd.z != 0) {
        Serial.print("z: "); Serial.println(cmd.x);
        motor_z.move(cmd.z);
      }
      g_isMoveNow = true;
    }
  }
}

void processHi() {
    Serial.print("version: ");
    Serial.print(MAJOR_VERSION);
    Serial.print(".");
    Serial.println(MINOR_VERSION);
    Serial.println("done");
}

void parseMoveSubCmd(String subcmd, MoveCmd &moveCmd);

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
  int16_t value = subcmd.substring(1).toInt();

  // скорость
  if ('s' == type) {
    moveCmd.s = value;;
  }
  else if ('x' == type) {
    moveCmd.x = value;
  }
  else if ('y' == type) {
    moveCmd.y = value;
  }
  else if ('z' == type) {
    moveCmd.z = value;
  }
  else if ('a' == type) {
    moveCmd.a = value;
  }
}
