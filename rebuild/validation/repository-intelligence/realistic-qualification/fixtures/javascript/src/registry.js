function sealed(value) {
  return value;
}

@sealed
export class Registry {
  find(name) {
    return name;
  }
}
